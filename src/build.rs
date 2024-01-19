#[cfg(feature = "podman-api")]
mod build_strategy;

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{self, Command},
};

use anyhow::{anyhow, bail, Result};
use clap::Args;
use log::{debug, error, info, trace, warn};
use typed_builder::TypedBuilder;
use users::{Users, UsersCache};

#[cfg(feature = "podman-api")]
use podman_api::{
    api::Image,
    opts::{ImageBuildOpts, ImageListOpts, ImagePushOpts, RegistryAuth},
    Podman,
};

#[cfg(feature = "podman-api")]
use build_strategy::BuildStrategy;

#[cfg(feature = "futures-util")]
use futures_util::StreamExt;

#[cfg(feature = "tokio")]
use tokio::runtime::Runtime;

use crate::{
    ops,
    template::{Recipe, TemplateCommand},
};

const LOCAL_BUILD: &str = "/etc/blue-build";

#[derive(Debug, Clone, Args, TypedBuilder)]
pub struct BuildCommand {
    /// The recipe file to build an image
    #[arg()]
    recipe: PathBuf,

    /// Rebase your current OS onto the image
    /// being built.
    ///
    /// This will create a tarball of your image at
    /// `/etc/blue-build/` and invoke `rpm-ostree` to
    /// rebase onto the image using `oci-archive`.
    ///
    /// NOTE: This can only be used if you have `rpm-ostree`
    /// installed and if the `--push` option isn't
    /// used. This image will not be signed.
    #[arg(short, long)]
    rebase: bool,

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
    #[builder(default, setter(into, strip_option))]
    registry: Option<String>,

    /// The url path to your base
    /// project images.
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    registry_path: Option<String>,

    /// The username to login to the
    /// container registry.
    #[arg(short = 'U', long)]
    #[builder(default, setter(into, strip_option))]
    username: Option<String>,

    /// The password to login to the
    /// container registry.
    #[arg(short = 'P', long)]
    #[builder(default, setter(into, strip_option))]
    password: Option<String>,

    /// The connection string used to connect
    /// to a remote podman socket.
    #[cfg(feature = "podman-api")]
    #[arg(short, long)]
    #[builder(default, setter(into, strip_option))]
    connection: Option<String>,

    /// The path to the `cert.pem`, `key.pem`,
    /// and `ca.pem` files needed to connect to
    /// a remote podman build socket.
    #[cfg(feature = "tls")]
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    tls_path: Option<PathBuf>,

    /// Whether to sign the image.
    #[cfg(feature = "sigstore")]
    #[arg(short, long)]
    #[builder(default)]
    sign: bool,

    /// Path to the public key used to sign the image.
    ///
    /// If the contents of the key are in an environment
    /// variable, you can use `env://` to sepcify which
    /// variable to read from.
    ///
    /// For example:
    ///
    /// bb build --public-key env://PUBLIC_KEY ...
    #[cfg(feature = "sigstore")]
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    public_key: Option<String>,

    /// Path to the private key used to sign the image.
    ///
    /// If the contents of the key are in an environment
    /// variable, you can use `env://` to sepcify which
    /// variable to read from.
    ///
    /// For example:
    ///
    /// bb build --private-key env://PRIVATE_KEY ...
    #[cfg(feature = "sigstore")]
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    private_key: Option<String>,
}

impl BuildCommand {
    /// Runs the command and returns a result.
    pub fn try_run(&mut self) -> Result<()> {
        trace!("BuildCommand::try_run()");

        if self.push && self.rebase {
            bail!("You cannot use '--rebase' and '--push' at the same time");
        }

        #[cfg(not(feature = "podman-api"))]
        if let Err(e1) = ops::check_command_exists("buildah") {
            ops::check_command_exists("podman").map_err(|e2| {
                anyhow!("Need either 'buildah' or 'podman' commands to proceed: {e1}, {e2}")
            })?;
        }

        if self.rebase {
            ops::check_command_exists("rpm-ostree")?;

            let cache = UsersCache::new();

            if cache.get_current_uid() != 0 {
                bail!("You need to be root to rebase a local image! Try using 'sudo'.");
            }

            clean_local_build_dir()?;
        }

        if self.push {
            ops::check_command_exists("cosign")?;
            ops::check_command_exists("skopeo")?;
            check_cosign_files()?;
        }

        TemplateCommand::builder()
            .recipe(self.recipe.clone())
            .output(PathBuf::from("Containerfile"))
            .build()
            .try_run()?;

        info!("Building image for recipe at {}", self.recipe.display());

        #[cfg(feature = "podman-api")]
        match BuildStrategy::determine_strategy()? {
            BuildStrategy::Socket(socket) => {
                let rt = Runtime::new()?;
                rt.block_on(self.build_image_podman_api(Podman::unix(socket)))
            }
            _ => self.build_image(),
        }

        #[cfg(not(feature = "podman-api"))]
        self.build_image()
    }

    /// Runs the command and exits if there is an error.
    pub fn run(&mut self) {
        trace!("BuildCommand::run()");

        if let Err(e) = self.try_run() {
            error!("Failed to build image: {e}");
            process::exit(1);
        }
    }

    #[cfg(feature = "podman-api")]
    async fn build_image_podman_api(&self, client: Podman) -> Result<()> {
        use podman_api::opts::ImageTagOpts;

        trace!("BuildCommand::build_image({client:#?})");

        let (registry, username, password) = if self.push {
            let registry = match (
                self.registry.as_ref(),
                env::var("CI_REGISTRY").ok(),
                env::var("GITHUB_ACTIONS").ok(),
            ) {
                (Some(registry), _, _) => registry.to_owned(),
                (None, Some(ci_registry), None) => ci_registry,
                (None, None, Some(_)) => "ghcr.io".to_string(),
                _ => bail!("Need '--registry' set in order to login"),
            };

            let username = match (
                self.username.as_ref(),
                env::var("CI_REGISTRY_USER").ok(),
                env::var("GITHUB_ACTOR").ok(),
            ) {
                (Some(username), _, _) => username.to_owned(),
                (None, Some(ci_registry_user), None) => ci_registry_user,
                (None, None, Some(github_actor)) => github_actor,
                _ => bail!("Need '--username' set in order to login"),
            };

            let password = match (
                self.password.as_ref(),
                env::var("CI_REGISTRY_PASSWORD").ok(),
                env::var("REGISTRY_TOKEN").ok(),
            ) {
                (Some(password), _, _) => password.to_owned(),
                (None, Some(ci_registry_password), None) => ci_registry_password,
                (None, None, Some(registry_token)) => registry_token,
                _ => bail!("Need '--password' set in order to login"),
            };

            (registry, username, password)
        } else {
            Default::default()
        };

        let recipe: Recipe = serde_yaml::from_str(fs::read_to_string(&self.recipe)?.as_str())?;
        trace!("recipe: {recipe:#?}");

        // Get values for image
        let tags = recipe.generate_tags();
        let image_name = self.generate_full_image_name(&recipe)?;
        let first_image_name = if tags.is_empty() || self.rebase {
            image_name.clone()
        } else {
            format!("{}:{}", &image_name, &tags[0])
        };
        debug!("Full tag is {first_image_name}");

        // Get podman ready to build
        let opts = ImageBuildOpts::builder(".")
            .tag(&first_image_name)
            .dockerfile("Containerfile")
            .remove(true)
            .layers(true)
            .pull(true)
            .build();
        trace!("Build options: {opts:#?}");

        match client.images().build(&opts) {
            Ok(mut build_stream) => {
                while let Some(chunk) = build_stream.next().await {
                    match chunk {
                        Ok(chunk) => debug!("{}", chunk.stream.trim()),
                        Err(e) => error!("{}", e),
                    }
                }
            }
            Err(e) => error!("{}", e),
        };

        if self.push {
            debug!("Pushing is enabled");

            trace!("cosign login -u {username} -p [MASKED] {registry}");
            if !Command::new("cosign")
                .arg("login")
                .arg("-u")
                .arg(&username)
                .arg("-p")
                .arg(&password)
                .arg(&registry)
                .output()?
                .status
                .success()
            {
                bail!("Failed to login for cosign!");
            }
            info!("Cosign login success at {registry}");

            let first_image = client.images().get(&first_image_name);

            for tag in &tags {
                let full_image_name = format!("{image_name}:{tag}");

                first_image
                    .tag(&ImageTagOpts::builder().repo(&image_name).tag(tag).build())
                    .await?;
                debug!("Tagged image {full_image_name}");

                let new_image = client.images().get(&full_image_name);

                info!("Pushing {full_image_name}");
                match new_image
                    .push(
                        &ImagePushOpts::builder()
                            .tls_verify(true)
                            .auth(
                                RegistryAuth::builder()
                                    .username(&username)
                                    .password(&password)
                                    .server_address(&registry)
                                    .build(),
                            )
                            .build(),
                    )
                    .await
                {
                    Ok(_) => info!("Pushed {full_image_name} successfully!"),
                    Err(e) => bail!("Failed to push image: {e}"),
                }
            }

            self.sign_images(&image_name, &tags[0])?;
        } else if self.rebase {
            debug!("Rebasing onto locally built image {image_name}");

            if Command::new("rpm-ostree")
                .arg("rebase")
                .arg(format!("ostree-unverified-image:{first_image_name}"))
                .status()?
                .success()
            {
                info!("Successfully rebased to {first_image_name}");
            } else {
                bail!("Failed to rebase to {first_image_name}");
            }
        }
        Ok(())
    }

    fn build_image(&self) -> Result<()> {
        trace!("BuildCommand::build_image()");
        let recipe: Recipe = serde_yaml::from_str(fs::read_to_string(&self.recipe)?.as_str())?;

        let tags = recipe.generate_tags();

        let image_name = self.generate_full_image_name(&recipe)?;

        if self.push {
            self.login()?;
        }
        self.run_build(&image_name, &tags)?;

        info!("Build complete!");

        if self.rebase {
            info!("Be sure to restart your computer to use your new changes!");
        }

        Ok(())
    }

    fn login(&self) -> Result<()> {
        trace!("BuildCommand::login()");
        info!("Attempting to login to the registry");

        let registry = match (
            self.registry.as_ref(),
            env::var("CI_REGISTRY").ok(),
            env::var("GITHUB_ACTIONS").ok(),
        ) {
            (Some(registry), _, _) => registry.to_owned(),
            (None, Some(ci_registry), None) => ci_registry,
            (None, None, Some(_)) => "ghcr.io".to_string(),
            _ => bail!("Need '--registry' set in order to login"),
        };

        let username = match (
            self.username.as_ref(),
            env::var("CI_REGISTRY_USER").ok(),
            env::var("GITHUB_ACTOR").ok(),
        ) {
            (Some(username), _, _) => username.to_owned(),
            (None, Some(ci_registry_user), None) => ci_registry_user,
            (None, None, Some(github_actor)) => github_actor,
            _ => bail!("Need '--username' set in order to login"),
        };

        let password = match (
            self.password.as_ref(),
            env::var("CI_REGISTRY_PASSWORD").ok(),
            env::var("REGISTRY_TOKEN").ok(),
        ) {
            (Some(password), None, None) => password.to_owned(),
            (None, Some(ci_registry_password), None) => ci_registry_password,
            (None, None, Some(github_token)) => github_token,
            _ => bail!("Need '--password' set in order to login"),
        };

        if !match (
            ops::check_command_exists("buildah"),
            ops::check_command_exists("podman"),
        ) {
            (Ok(_), _) => {
                trace!("buildah login -u {username} -p [MASKED] {registry}");
                Command::new("buildah")
            }
            (Err(_), Ok(_)) => {
                trace!("podman login -u {username} -p [MASKED] {registry}");
                Command::new("podman")
            }
            _ => bail!("Need 'buildah' or 'podman' to login"),
        }
        .arg("login")
        .arg("-u")
        .arg(&username)
        .arg("-p")
        .arg(&password)
        .arg(&registry)
        .output()?
        .status
        .success()
        {
            bail!("Failed to login for buildah!");
        }

        trace!("cosign login -u {username} -p [MASKED] {registry}");
        if !Command::new("cosign")
            .arg("login")
            .arg("-u")
            .arg(&username)
            .arg("-p")
            .arg(&password)
            .arg(&registry)
            .output()?
            .status
            .success()
        {
            bail!("Failed to login for cosign!");
        }
        info!("Login success at {registry}");

        Ok(())
    }

    fn generate_full_image_name(&self, recipe: &Recipe) -> Result<String> {
        info!("Generating full image name");
        trace!("BuildCommand::generate_full_image_name({recipe:#?})");

        let image_name = if self.rebase {
            let local_build_path = PathBuf::from(LOCAL_BUILD);

            let image_path = local_build_path.join(format!("{}.tar.gz", &recipe.name));
            format!("oci-archive:{}", image_path.display())
        } else {
            match (
                env::var("CI_REGISTRY").ok(),
                env::var("CI_PROJECT_NAMESPACE").ok(),
                env::var("CI_PROJECT_NAME").ok(),
                env::var("GITHUB_REPOSITORY_OWNER").ok(),
                self.registry.as_ref(),
                self.registry_path.as_ref(),
            ) {
                (_, _, _, _, Some(registry), Some(registry_path)) => {
                    trace!("registry={registry}, registry_path={registry_path}");
                    format!(
                        "{}/{}/{}",
                        registry.trim().trim_matches('/'),
                        registry_path.trim().trim_matches('/'),
                        &recipe.name
                    )
                }
                (
                    Some(ci_registry),
                    Some(ci_project_namespace),
                    Some(ci_project_name),
                    None,
                    None,
                    None,
                ) => {
                    trace!("CI_REGISTRY={ci_registry}, CI_PROJECT_NAMESPACE={ci_project_namespace}, CI_PROJECT_NAME={ci_project_name}");
                    warn!("Generating Gitlab Registry image");
                    format!(
                        "{ci_registry}/{ci_project_namespace}/{ci_project_name}/{}",
                        &recipe.name
                    )
                }
                (None, None, None, Some(github_repository_owner), None, None) => {
                    trace!("GITHUB_REPOSITORY_OWNER={github_repository_owner}");
                    warn!("Generating Github Registry image");
                    format!("ghcr.io/{github_repository_owner}/{}", &recipe.name)
                }
                _ => {
                    trace!("Nothing to indicate an image name with a registry");
                    if self.push {
                        bail!("Need '--registry' and '--registry-path' in order to push image");
                    }
                    recipe.name.to_owned()
                }
            }
        };

        info!("Using image name '{image_name}'");

        Ok(image_name)
    }

    fn run_build(&self, image_name: &str, tags: &[String]) -> Result<()> {
        trace!("BuildCommand::run_build({image_name}, {tags:#?})");

        let mut tags_iter = tags.iter();

        let first_tag = tags_iter
            .next()
            .ok_or(anyhow!("We got here with no tags!?"))?;

        let full_image = if self.rebase {
            image_name.to_owned()
        } else {
            format!("{image_name}:{first_tag}")
        };

        let status = match (
            ops::check_command_exists("buildah"),
            ops::check_command_exists("podman"),
        ) {
            (Ok(_), _) => {
                trace!("buildah build -t {full_image}");
                Command::new("buildah")
                    .arg("build")
                    .arg("-t")
                    .arg(&full_image)
                    .status()?
            }
            (Err(_), Ok(_)) => {
                trace!("podman build . -t {full_image}");
                Command::new("podman")
                    .arg("build")
                    .arg(".")
                    .arg("-t")
                    .arg(&full_image)
                    .status()?
            }
            (Err(e1), Err(e2)) => bail!("Need either 'buildah' or 'podman' to build: {e1}, {e2}"),
        };

        if status.success() {
            info!("Successfully built {image_name}");
        } else {
            bail!("Failed to build {image_name}");
        }

        if tags.len() > 1 && !self.rebase {
            debug!("Tagging all images");

            for tag in tags_iter {
                debug!("Tagging {image_name} with {tag}");

                let tag_image = format!("{image_name}:{tag}");

                let status = match (
                    ops::check_command_exists("buildah"),
                    ops::check_command_exists("podman"),
                ) {
                    (Ok(_), _) => {
                        trace!("buildah tag {full_image} {tag_image}");
                        Command::new("buildah")
                    }
                    (Err(_), Ok(_)) => {
                        trace!("podman tag {full_image} {tag_image}");
                        Command::new("podman")
                    }
                    (Err(e1), Err(e2)) => {
                        bail!("Need either 'buildah' or 'podman' to build: {e1}, {e2}")
                    }
                }
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

                let status = match (
                    ops::check_command_exists("buildah"),
                    ops::check_command_exists("podman"),
                ) {
                    (Ok(_), _) => {
                        trace!("buildah push {tag_image}");
                        Command::new("buildah")
                    }
                    (Err(_), Ok(_)) => {
                        trace!("podman push {tag_image}");
                        Command::new("podman")
                    }
                    (Err(e1), Err(e2)) => {
                        bail!("Need either 'buildah' or 'podman' to build: {e1}, {e2}")
                    }
                }
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
        } else if self.rebase {
            debug!("Rebasing onto locally built image {image_name}");

            if Command::new("rpm-ostree")
                .arg("rebase")
                .arg(format!("ostree-unverified-image:{full_image}"))
                .status()?
                .success()
            {
                info!("Successfully rebased to {full_image}");
            } else {
                bail!("Failed to rebase to {full_image}");
            }
        }

        Ok(())
    }

    fn sign_images(&self, image_name: &str, tag: &str) -> Result<()> {
        trace!("BuildCommand::sign_images({image_name}, {tag})");

        env::set_var("COSIGN_PASSWORD", "");
        env::set_var("COSIGN_YES", "true");

        let image_digest = get_image_digest(image_name, tag)?;

        match (
            env::var("CI_DEFAULT_BRANCH"),
            env::var("CI_COMMIT_REF_NAME"),
            env::var("CI_PROJECT_URL"),
            env::var("CI_SERVER_PROTOCOL"),
            env::var("CI_SERVER_HOST"),
            env::var("SIGSTORE_ID_TOKEN"),
            env::var("GITHUB_EVENT_NAME"),
            env::var("GITHUB_REF_NAME"),
            env::var("COSIGN_PRIVATE_KEY"),
        ) {
            (
                Ok(ci_default_branch),
                Ok(ci_commit_ref),
                Ok(ci_project_url),
                Ok(ci_server_protocol),
                Ok(ci_server_host),
                Ok(_),
                _,
                _,
                _,
            ) if ci_default_branch == ci_commit_ref => {
                trace!("CI_PROJECT_URL={ci_project_url}, CI_DEFAULT_BRANCH={ci_default_branch}, CI_COMMIT_REF_NAME={ci_commit_ref}, CI_SERVER_PROTOCOL={ci_server_protocol}, CI_SERVER_HOST={ci_server_host}");

                debug!("On default branch");

                info!("Signing image: {image_digest}");

                trace!("cosign sign {image_digest}");

                if Command::new("cosign")
                    .arg("sign")
                    .arg(&image_digest)
                    .status()?
                    .success()
                {
                    info!("Successfully signed image!");
                } else {
                    bail!("Failed to sign image: {image_digest}");
                }

                let cert_ident =
                    format!("{ci_project_url}//.gitlab-ci.yml@refs/heads/{ci_default_branch}");

                let cert_oidc = format!("{ci_server_protocol}://{ci_server_host}");

                trace!("cosign verify --certificate-identity {cert_ident} --certificate-oidc-issuer {cert_oidc} {image_name}:{tag}");

                if !Command::new("cosign")
                    .arg("verify")
                    .arg("--certificate-identity")
                    .arg(&cert_ident)
                    .arg("--certificate-oidc-issuer")
                    .arg(&cert_oidc)
                    .arg(&format!("{image_name}:{tag}"))
                    .status()?
                    .success()
                {
                    bail!("Failed to verify image!");
                }
            }
            (_, _, _, _, _, _, Ok(github_event_name), Ok(github_ref_name), Ok(_))
                if github_event_name != "pull_request" && github_ref_name == "live" =>
            {
                trace!("GITHUB_EVENT_NAME={github_event_name}, GITHUB_REF_NAME={github_ref_name}");

                debug!("On live branch");

                info!("Signing image: {image_digest}");

                trace!("cosign sign --key=env://COSIGN_PRIVATE_KEY {image_digest}");

                if Command::new("cosign")
                    .arg("sign")
                    .arg("--key=env://COSIGN_PRIVATE_KEY")
                    .arg(&image_digest)
                    .status()?
                    .success()
                {
                    info!("Successfully signed image!");
                } else {
                    bail!("Failed to sign image: {image_digest}");
                }

                trace!("cosign verify --key ./cosign.pub {image_name}:{tag}");

                if !Command::new("cosign")
                    .arg("verify")
                    .arg("--key=./cosign.pub")
                    .arg(&format!("{image_name}:{tag}"))
                    .status()?
                    .success()
                {
                    bail!("Failed to verify image!");
                }
            }
            _ => debug!("Not running in CI with cosign variables, not signing"),
        }

        Ok(())
    }
}

fn get_image_digest(image_name: &str, tag: &str) -> Result<String> {
    trace!("get_image_digest({image_name}, {tag})");

    let image_url = format!("docker://{image_name}:{tag}");

    trace!("skopeo inspect --format='{{.Digest}}' {image_url}");
    let image_digest = String::from_utf8(
        Command::new("skopeo")
            .arg("inspect")
            .arg("--format='{{.Digest}}'")
            .arg(&image_url)
            .output()?
            .stdout,
    )?;

    Ok(format!(
        "{image_name}@{}",
        image_digest.trim().trim_matches('\'')
    ))
}

fn check_cosign_files() -> Result<()> {
    trace!("check_for_cosign_files()");

    match (
        env::var("GITHUB_EVENT_NAME").ok(),
        env::var("GITHUB_REF_NAME").ok(),
        env::var("COSIGN_PRIVATE_KEY").ok(),
    ) {
        (Some(github_event_name), Some(github_ref), Some(_))
            if github_event_name != "pull_request" && github_ref == "live" =>
        {
            env::set_var("COSIGN_PASSWORD", "");
            env::set_var("COSIGN_YES", "true");

            debug!("Building on live branch, checking cosign files");
            trace!("cosign public-key --key env://COSIGN_PRIVATE_KEY");
            let output = Command::new("cosign")
                .arg("public-key")
                .arg("--key=env://COSIGN_PRIVATE_KEY")
                .output()?;

            if !output.status.success() {
                bail!(
                    "Failed to run cosign public-key: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
            }

            let calculated_pub_key = String::from_utf8(output.stdout)?;
            let found_pub_key = fs::read_to_string("./cosign.pub")?;
            trace!("calculated_pub_key={calculated_pub_key},found_pub_key={found_pub_key}");

            if calculated_pub_key.trim() == found_pub_key.trim() {
                debug!("Cosign files match, continuing build");
                Ok(())
            } else {
                bail!("Public key 'cosign.pub' does not match private key")
            }
        }
        _ => {
            debug!("Not building on live branch, skipping cosign file check");
            Ok(())
        }
    }
}

fn clean_local_build_dir() -> Result<()> {
    trace!("clean_local_build_dir()");
    let local_build_path = Path::new(LOCAL_BUILD);

    if !local_build_path.exists() {
        trace!(
            "Creating build output dir at {}",
            local_build_path.display()
        );
        fs::create_dir_all(local_build_path)?;
    } else {
        debug!("Cleaning out build dir {LOCAL_BUILD}");

        let entries = fs::read_dir(LOCAL_BUILD)?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            trace!("Found {}", path.display());

            if path.is_file() && path.ends_with(".tar.gz") {
                trace!("Removing {}", path.display());
                fs::remove_file(path)?;
            }
        }
    }

    Ok(())
}
