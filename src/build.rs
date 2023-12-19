use std::{
    env, fs,
    path::PathBuf,
    process::{self, Command},
};

use anyhow::{anyhow, bail, Result};
use chrono::Local;
use clap::Args;
use log::{debug, error, info, trace, warn};
use typed_builder::TypedBuilder;

use crate::{module_recipe::Recipe, ops, template::TemplateCommand};

#[derive(Debug, Clone, Args, TypedBuilder)]
pub struct BuildCommand {
    /// The recipe file to build an image
    #[arg()]
    recipe: PathBuf,

    /// Optional Containerfile to use as a template
    #[arg(short, long)]
    #[builder(default, setter(into))]
    containerfile: Option<PathBuf>,

    /// Push the image with all the tags.
    ///
    /// Requires `--registry`, `--registry-path`,
    /// `--username`, and `--password` if not
    /// building in CI.
    #[arg(short, long)]
    #[builder(default)]
    push: bool,

    /// The registry's domain name.
    #[arg(long)]
    #[builder(default, setter(into))]
    registry: Option<String>,

    /// The url path to your base
    /// project images.
    #[arg(long)]
    #[builder(default, setter(into))]
    registry_path: Option<String>,

    /// The username to login to the
    /// container registry.
    #[arg(short, long)]
    #[builder(default, setter(into))]
    username: Option<String>,

    /// The password to login to the
    /// container registry.
    #[arg(short, long)]
    #[builder(default, setter(into))]
    password: Option<String>,
}

impl BuildCommand {
    pub fn run(&self) -> Result<()> {
        info!("Templating for recipe at {}", self.recipe.display());

        if let Err(e) = TemplateCommand::builder()
            .recipe(self.recipe.clone())
            .containerfile(self.containerfile.clone())
            .output(PathBuf::from("Containerfile"))
            .build()
            .run()
        {
            error!("Failed to template file: {e}");
            process::exit(1);
        }

        info!("Building image for recipe at {}", self.recipe.display());
        if let Err(e) = self.build_image() {
            error!("Failed to build image: {e}");
            process::exit(1);
        }

        Ok(())
    }

    fn build_image(&self) -> Result<()> {
        trace!("BuildCommand::build_image()");
        ops::check_command_exists("buildah")?;

        if self.push {
            ops::check_command_exists("cosign")?;
            ops::check_command_exists("skopeo")?;
        }

        let recipe: Recipe = serde_yaml::from_str(fs::read_to_string(&self.recipe)?.as_str())?;

        let tags = self.generate_tags(&recipe);

        let image_name = self.generate_full_image_name(&recipe)?;

        if self.push {
            self.login()?;
        }
        self.run_build(&image_name, &tags)?;

        info!("Build complete!");

        Ok(())
    }

    fn generate_tags(&self, recipe: &Recipe) -> Vec<String> {
        debug!("Generating image tags for {}", &recipe.name);
        trace!("BuildCommand::generate_tags({recipe:#?})");

        let mut tags: Vec<String> = Vec::new();
        let image_version = recipe.image_version;
        let timestamp = Local::now().format("%Y%m%d").to_string();

        if env::var("CI").is_ok() {
            warn!("Detected running in Gitlab, pulling information from CI variables");

            if let (Ok(mr_iid), Ok(pipeline_source)) = (
                env::var("CI_MERGE_REQUEST_IID"),
                env::var("CI_PIPELINE_SOURCE"),
            ) {
                trace!("CI_MERGE_REQUEST_IID={mr_iid}, CI_PIPELINE_SOURCE={pipeline_source}");
                if pipeline_source == "merge_request_event" {
                    debug!("Running in a MR");
                    tags.push(format!("{mr_iid}-{image_version}"));
                }
            }

            if let Ok(commit_sha) = env::var("CI_COMMIT_SHORT_SHA") {
                trace!("CI_COMMIT_SHORT_SHA={commit_sha}");
                tags.push(format!("{commit_sha}-{image_version}"));
            }

            if let (Ok(commit_branch), Ok(default_branch)) = (
                env::var("CI_COMMIT_REF_NAME"),
                env::var("CI_DEFAULT_BRANCH"),
            ) {
                trace!("CI_COMMIT_REF_NAME={commit_branch}, CI_DEFAULT_BRANCH={default_branch}");
                if default_branch != commit_branch {
                    debug!("Running on branch {commit_branch}");
                    tags.push(format!("br-{commit_branch}-{image_version}"));
                } else {
                    debug!("Running on the default branch");
                    tags.push(image_version.to_string());
                    tags.push(format!("{image_version}-{timestamp}"));
                    tags.push(timestamp.to_string());
                }
            }
        } else {
            warn!("Running locally");
            tags.push(format!("{image_version}-local"));
        }
        info!("Finished generating tags!");
        trace!("Tags: {tags:#?}");
        tags
    }

    fn login(&self) -> Result<()> {
        info!("Attempting to login to the registry");
        trace!("BuildCommand::login()");

        let registry = match &self.registry {
            Some(registry) => registry.to_owned(),
            None => env::var("CI_REGISTRY")?,
        };

        let username = match &self.username {
            Some(username) => username.to_owned(),
            None => env::var("CI_REGISTRY_USER")?,
        };

        let password = match &self.password {
            Some(password) => password.to_owned(),
            None => env::var("CI_REGISTRY_PASSWORD")?,
        };

        trace!("buildah login -u {username} -p [MASKED] {registry}");
        match Command::new("buildah")
            .arg("login")
            .arg("-u")
            .arg(&username)
            .arg("-p")
            .arg(&password)
            .arg(&registry)
            .status()?
            .success()
        {
            true => info!("Buildah login success at {registry} for user {username}!"),
            false => return Err(anyhow!("Failed to login for buildah!")),
        }

        trace!("cosign login -u {username} -p [MASKED] {registry}");
        match Command::new("cosign")
            .arg("login")
            .arg("-u")
            .arg(&username)
            .arg("-p")
            .arg(&password)
            .arg(&registry)
            .status()?
            .success()
        {
            true => info!("Cosign login success at {registry} for user {username}!"),
            false => return Err(anyhow!("Failed to login for cosign!")),
        }

        Ok(())
    }

    fn generate_full_image_name(&self, recipe: &Recipe) -> Result<String> {
        info!("Generating full image name");
        trace!("BuildCommand::generate_full_image_name({recipe:#?})");

        let image_name = recipe.name.as_str();

        let image_name = if env::var("CI").is_ok() {
            warn!("Detected running in Gitlab CI");
            if let (Ok(registry), Ok(project_namespace), Ok(project_name)) = (
                env::var("CI_REGISTRY"),
                env::var("CI_PROJECT_NAMESPACE"),
                env::var("CI_PROJECT_NAME"),
            ) {
                trace!("CI_REGISTRY={registry}, CI_PROJECT_NAMESPACE={project_namespace}, CI_PROJECT_NAME={project_name}");
                format!("{registry}/{project_namespace}/{project_name}/{image_name}")
            } else {
                bail!("Unable to generate image name for Gitlab CI env!")
            }
        } else {
            warn!("Detected running locally");
            if let (Some(registry), Some(registry_path)) =
                (self.registry.as_ref(), self.registry_path.as_ref())
            {
                format!(
                    "{}/{}/{image_name}",
                    registry.trim_matches('/'),
                    registry_path.trim_matches('/')
                )
            } else {
                if self.push {
                    bail!("Need '--registry' and '--registry-path' in order to push image");
                }
                image_name.to_string()
            }
        };

        info!("Using image name {image_name}");

        Ok(image_name)
    }

    fn run_build(&self, image_name: &str, tags: &[String]) -> Result<()> {
        trace!("BuildCommand::run_build({image_name}, {tags:#?})");

        let mut tags_iter = tags.iter();

        let first_tag = tags_iter
            .next()
            .ok_or(anyhow!("We got here with no tags!?"))?;

        let full_image = format!("{image_name}:{first_tag}");

        trace!("buildah build -t {full_image}");
        let status = Command::new("buildah")
            .arg("build")
            .arg("-t")
            .arg(&full_image)
            .status()?;

        if status.success() {
            info!("Successfully built {image_name}");
        } else {
            bail!("Failed to build {image_name}");
        }

        if tags.len() > 1 {
            debug!("Tagging all images");

            for tag in tags_iter {
                debug!("Tagging {image_name} with {tag}");

                let tag_image = format!("{image_name}:{tag}");

                trace!("buildah tag {full_image} {tag_image}");
                let status = Command::new("buildah")
                    .arg("tag")
                    .arg(&full_image)
                    .arg(&tag_image)
                    .status()?;

                if status.success() {
                    info!("Successfully tagged {image_name}:{tag}!");
                } else {
                    bail!("Failed to tag image {image_name}:{tag}");
                }
            }
        }

        if self.push {
            debug!("Pushing all images");
            for tag in tags.iter() {
                debug!("Pushing image {image_name}:{tag}");

                let tag_image = format!("{image_name}:{tag}");

                trace!("buildah push {tag_image}");
                let status = Command::new("buildah")
                    .arg("push")
                    .arg(&tag_image)
                    .status()?;

                if status.success() {
                    info!("Successfully pushed {image_name}:{tag}!")
                } else {
                    bail!("Failed to push image {image_name}:{tag}");
                }
            }

            self.sign_images(image_name, first_tag)?;
        }

        Ok(())
    }

    fn sign_images(&self, image_name: &str, tag: &str) -> Result<()> {
        trace!("BuildCommand::sign_images({image_name}, {tag})");

        if env::var("SIGSTORE_ID_TOKEN").is_ok() && env::var("CI").is_ok() {
            debug!("SIGSTORE_ID_TOKEN detected, signing image");

            if let (
                Ok(project_url),
                Ok(default_branch),
                Ok(commit_branch),
                Ok(server_protocol),
                Ok(server_host),
            ) = (
                env::var("CI_PROJECT_URL"),
                env::var("CI_DEFAULT_BRANCH"),
                env::var("CI_COMMIT_REF_NAME"),
                env::var("CI_SERVER_PROTOCOL"),
                env::var("CI_SERVER_HOST"),
            ) {
                trace!("CI_PROJECT_URL={project_url}, CI_DEFAULT_BRANCH={default_branch}, CI_COMMIT_REF_NAME={commit_branch}, CI_SERVER_PROTOCOL={server_protocol}, CI_SERVER_HOST={server_host}");

                if default_branch == commit_branch {
                    debug!("On default branch, retrieving image digest");

                    let image_name_tag = format!("{image_name}:{tag}");
                    let image_url = format!("docker://{image_name_tag}");

                    trace!("skopeo inspect --format='{{.Digest}}' {image_url}");
                    let image_digest = String::from_utf8(
                        Command::new("skopeo")
                            .arg("inspect")
                            .arg("--format='{{.Digest}}'")
                            .arg(&image_url)
                            .output()?
                            .stdout,
                    )?;

                    let image_digest =
                        format!("{image_name}@{}", image_digest.trim().trim_matches('\''));

                    info!("Signing image: {image_digest}");

                    env::set_var("COSIGN_PASSWORD", "");
                    env::set_var("COSIGN_YES", "true");

                    trace!("cosign sign {image_digest}");
                    let status = Command::new("cosign")
                        .arg("sign")
                        .arg(&image_digest)
                        .status()?;

                    if status.success() {
                        info!("Successfully signed image!");
                    } else {
                        bail!("Failed to sign image: {image_digest}");
                    }

                    let cert_ident =
                        format!("{project_url}//.gitlab-ci.yml@refs/heads/{default_branch}");

                    let cert_oidc = format!("{server_protocol}://{server_host}");

                    trace!("cosign verify --certificate-identity {cert_ident}");
                    let status = Command::new("cosign")
                        .arg("verify")
                        .arg("--certificate-identity")
                        .arg(&cert_ident)
                        .arg("--certificate-oidc-issuer")
                        .arg(&cert_oidc)
                        .arg(&image_name_tag)
                        .status()?;

                    if !status.success() {
                        bail!("Failed to verify image!");
                    }
                }
            }
        } else {
            debug!("No SIGSTORE_ID_TOKEN detected, not signing image");
        }

        Ok(())
    }
}
