use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{bail, Result};
use blue_build_recipe::Recipe;
use blue_build_utils::constants::*;
use clap::Args;
use colorized::{Color, Colors};
use log::{debug, info, trace, warn};
use typed_builder::TypedBuilder;

use crate::{commands::template::TemplateCommand, strategies::Strategy};

use super::BlueBuildCommand;

#[derive(Debug, Clone, Args, TypedBuilder)]
pub struct BuildCommand {
    /// The recipe file to build an image
    #[arg()]
    #[builder(default, setter(into, strip_option))]
    recipe: Option<PathBuf>,

    /// Push the image with all the tags.
    ///
    /// Requires `--registry`,
    /// `--username`, and `--password` if not
    /// building in CI.
    #[arg(short, long)]
    #[builder(default)]
    push: bool,

    /// Block `bluebuild` from retrying to push the image.
    #[arg(short, long, default_value_t = true)]
    #[builder(default)]
    no_retry_push: bool,

    /// The number of times to retry pushing the image.
    #[arg(long, default_value_t = 1)]
    #[builder(default)]
    retry_count: u8,

    /// Allow `bluebuild` to overwrite an existing
    /// Containerfile without confirmation.
    ///
    /// This is not needed if the Containerfile is in
    /// .gitignore or has already been built by `bluebuild`.
    #[arg(short, long)]
    #[builder(default)]
    force: bool,

    /// Archives the built image into a tarfile
    /// in the specified directory.
    #[arg(short, long)]
    #[builder(default, setter(into, strip_option))]
    archive: Option<PathBuf>,

    /// The registry's domain name.
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    registry: Option<String>,

    /// The url path to your base
    /// project images.
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    #[arg(visible_alias("registry-path"))]
    registry_namespace: Option<String>,

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
    #[cfg(feature = "tls")]
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
    /// bluebuild build --public-key env://PUBLIC_KEY ...
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
    /// bluebuild build --private-key env://PRIVATE_KEY ...
    #[cfg(feature = "sigstore")]
    #[arg(long)]
    #[builder(default, setter(into, strip_option))]
    private_key: Option<String>,
}

impl BlueBuildCommand for BuildCommand {
    /// Runs the command and returns a result.
    fn try_run(&mut self) -> Result<()> {
        trace!("BuildCommand::try_run()");

        Strategy::builder()
            .username(self.username.as_ref())
            .password(self.password.as_ref())
            .registry(self.registry.as_ref())
            .build()
            .init()?;

        // Check if the Containerfile exists
        //   - If doesn't => *Build*
        //   - If it does:
        //     - check entry in .gitignore
        //       -> If it is => *Build*
        //       -> If isn't:
        //         - check if it has the BlueBuild tag (LABEL)
        //           -> If it does => *Ask* to add to .gitignore and remove from git
        //           -> If it doesn't => *Ask* to continue and override the file

        let container_file_path = Path::new(CONTAINER_FILE);

        if !self.force && container_file_path.exists() {
            let gitignore = fs::read_to_string(GITIGNORE_PATH)?;

            let is_ignored = gitignore
                .lines()
                .any(|line: &str| line.contains(CONTAINER_FILE));

            if !is_ignored {
                let containerfile = fs::read_to_string(container_file_path)?;
                let has_label = containerfile.lines().any(|line| {
                    let label = format!("LABEL {}", BUILD_ID_LABEL);
                    line.to_string().trim().starts_with(&label)
                });

                let question = requestty::Question::confirm("build")
                    .message(
                        if has_label {
                            LABELED_ERROR_MESSAGE
                        } else {
                            NO_LABEL_ERROR_MESSAGE
                        }
                        .color(Colors::BrightYellowFg),
                    )
                    .default(true)
                    .build();

                if let Ok(answer) = requestty::prompt_one(question) {
                    if answer.as_bool().unwrap_or(false) {
                        blue_build_utils::append_to_file(
                            GITIGNORE_PATH,
                            &format!("/{}", CONTAINER_FILE),
                        )?;
                    }
                }
            }
        }

        if self.push && self.archive.is_some() {
            bail!("You cannot use '--archive' and '--push' at the same time");
        }

        let recipe_path = self
            .recipe
            .clone()
            .unwrap_or_else(|| PathBuf::from(RECIPE_PATH));

        TemplateCommand::builder()
            .recipe(&recipe_path)
            .output(PathBuf::from("Containerfile"))
            .build()
            .try_run()?;

        if self.push {
            blue_build_utils::check_command_exists("cosign")?;
            blue_build_utils::check_command_exists("skopeo")?;
            check_cosign_files()?;
        }

        info!("Building image for recipe at {}", recipe_path.display());

        self.start(&recipe_path)
    }
}

impl BuildCommand {
    fn start(&self, recipe_path: &Path) -> Result<()> {
        trace!("BuildCommand::build_image()");

        let recipe = Recipe::parse(&recipe_path)?;
        let os_version = Strategy::get_os_version(&recipe)?;
        let tags = recipe.generate_tags(&os_version);
        let image_name = self.generate_full_image_name(&recipe)?;

        if self.push {
            self.login()?;
        }

        self.run_build(&image_name, &tags)?;

        info!("Build complete!");

        Ok(())
    }

    fn login(&self) -> Result<()> {
        trace!("BuildCommand::login()");
        info!("Attempting to login to the registry");

        let credentials = Strategy::get_credentials()?;

        let (registry, username, password) = (
            &credentials.registry,
            &credentials.username,
            &credentials.password,
        );

        info!("Logging into the registry, {registry}");
        Strategy::get_build_strategy().login()?;

        trace!("cosign login -u {username} -p [MASKED] {registry}");
        let login_output = Command::new("cosign")
            .arg("login")
            .arg("-u")
            .arg(username)
            .arg("-p")
            .arg(password)
            .arg(registry)
            .output()?;

        if !login_output.status.success() {
            let err_output = String::from_utf8_lossy(&login_output.stderr);
            bail!("Failed to login for cosign: {err_output}");
        }
        info!("Login success at {registry}");

        Ok(())
    }

    /// # Errors
    ///
    /// Will return `Err` if the image name cannot be generated.
    pub fn generate_full_image_name(&self, recipe: &Recipe) -> Result<String> {
        trace!("BuildCommand::generate_full_image_name({recipe:#?})");
        info!("Generating full image name");

        let image_name = if let Some(archive_dir) = &self.archive {
            format!(
                "oci-archive:{}/{}.{ARCHIVE_SUFFIX}",
                archive_dir.to_string_lossy().trim_end_matches('/'),
                recipe.name.to_lowercase().replace('/', "_"),
            )
        } else {
            match (
                env::var(CI_REGISTRY).ok().map(|s| s.to_lowercase()),
                env::var(CI_PROJECT_NAMESPACE)
                    .ok()
                    .map(|s| s.to_lowercase()),
                env::var(CI_PROJECT_NAME).ok().map(|s| s.to_lowercase()),
                env::var(GITHUB_REPOSITORY_OWNER)
                    .ok()
                    .map(|s| s.to_lowercase()),
                self.registry.as_ref().map(|s| s.to_lowercase()),
                self.registry_namespace.as_ref().map(|s| s.to_lowercase()),
            ) {
                (_, _, _, _, Some(registry), Some(registry_path)) => {
                    trace!("registry={registry}, registry_path={registry_path}");
                    format!(
                        "{}/{}/{}",
                        registry.trim().trim_matches('/'),
                        registry_path.trim().trim_matches('/'),
                        recipe.name.trim(),
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
                        recipe.name.trim().to_lowercase()
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
                    recipe.name.trim().to_lowercase()
                }
            }
        };

        debug!("Using image name '{image_name}'");

        Ok(image_name)
    }

    /// # Errors
    ///
    /// Will return `Err` if the build fails.
    fn run_build(&self, image_name: &str, tags: &[String]) -> Result<()> {
        trace!("BuildCommand::run_build({image_name}, {tags:#?})");

        let strat = Strategy::get_build_strategy();

        let full_image = if self.archive.is_some() {
            image_name.to_string()
        } else {
            tags.first()
                .map_or_else(|| image_name.to_string(), |t| format!("{image_name}:{t}"))
        };

        info!("Building image {full_image}");
        strat.build(&full_image)?;

        if tags.len() > 1 && self.archive.is_none() {
            debug!("Tagging all images");

            for tag in tags {
                debug!("Tagging {image_name} with {tag}");

                strat.tag(&full_image, image_name, tag)?;

                if self.push {
                    let retry_count = if !self.no_retry_push {
                        self.retry_count
                    } else {
                        0
                    };

                    debug!("Pushing all images");
                    // Push images with retries (1s delay between retries)
                    blue_build_utils::retry(retry_count, 1000, || {
                        debug!("Pushing image {image_name}:{tag}");

                        let tag_image = format!("{image_name}:{tag}");

                        strat.push(&tag_image)
                    })?;
                }
            }
        }

        if self.push {
            sign_images(image_name, tags.first().map(String::as_str))?;
        }

        Ok(())
    }
}

// ======================================================== //
// ========================= Helpers ====================== //
// ======================================================== //

fn sign_images(image_name: &str, tag: Option<&str>) -> Result<()> {
    trace!("BuildCommand::sign_images({image_name}, {tag:?})");

    env::set_var("COSIGN_PASSWORD", "");
    env::set_var("COSIGN_YES", "true");

    let image_digest = get_image_digest(image_name, tag)?;
    let image_name_tag = tag.map_or_else(|| image_name.to_owned(), |t| format!("{image_name}:{t}"));

    match (
        // GitLab specific vars
        env::var(CI_DEFAULT_BRANCH),
        env::var(CI_PROJECT_URL),
        env::var(CI_SERVER_PROTOCOL),
        env::var(CI_SERVER_HOST),
        env::var(SIGSTORE_ID_TOKEN),
        // GitHub specific vars
        env::var(GITHUB_TOKEN),
        env::var(GITHUB_WORKFLOW_REF),
        // Cosign public/private key pair
        env::var(COSIGN_PRIVATE_KEY),
    ) {
        // Cosign public/private key pair
        (_, _, _, _, _, _, _, Ok(cosign_private_key))
            if !cosign_private_key.is_empty() && Path::new(COSIGN_PATH).exists() =>
        {
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

            trace!("cosign verify --key {COSIGN_PATH} {image_name_tag}");

            if !Command::new("cosign")
                .arg("verify")
                .arg(format!("--key={COSIGN_PATH}"))
                .arg(&image_name_tag)
                .status()?
                .success()
            {
                bail!("Failed to verify image!");
            }
        }
        // Gitlab keyless
        (
            Ok(ci_default_branch),
            Ok(ci_project_url),
            Ok(ci_server_protocol),
            Ok(ci_server_host),
            Ok(_),
            _,
            _,
            _,
        ) => {
            trace!("CI_PROJECT_URL={ci_project_url}, CI_DEFAULT_BRANCH={ci_default_branch}, CI_SERVER_PROTOCOL={ci_server_protocol}, CI_SERVER_HOST={ci_server_host}");

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

            trace!("cosign verify --certificate-identity {cert_ident} --certificate-oidc-issuer {cert_oidc} {image_name_tag}");

            if !Command::new("cosign")
                .arg("verify")
                .arg("--certificate-identity")
                .arg(&cert_ident)
                .arg("--certificate-oidc-issuer")
                .arg(&cert_oidc)
                .arg(&image_name_tag)
                .status()?
                .success()
            {
                bail!("Failed to verify image!");
            }
        }
        // GitHub keyless
        (_, _, _, _, _, Ok(_), Ok(github_worflow_ref), _) => {
            trace!("GITHUB_WORKFLOW_REF={github_worflow_ref}");

            info!("Signing image {image_digest}");

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

            trace!("cosign verify --certificate-identity-regexp {github_worflow_ref} --certificate-oidc-issuer {GITHUB_TOKEN_ISSUER_URL} {image_name_tag}");
            if !Command::new("cosign")
                .arg("verify")
                .arg("--certificate-identity-regexp")
                .arg(&github_worflow_ref)
                .arg("--certificate-oidc-issuer")
                .arg(GITHUB_TOKEN_ISSUER_URL)
                .arg(&image_name_tag)
                .status()?
                .success()
            {
                bail!("Failed to verify image!");
            }
        }
        _ => warn!("Not running in CI with cosign variables, not signing"),
    }

    Ok(())
}

fn get_image_digest(image_name: &str, tag: Option<&str>) -> Result<String> {
    trace!("get_image_digest({image_name}, {tag:?})");

    let image_url = tag.map_or_else(
        || format!("docker://{image_name}"),
        |tag| format!("docker://{image_name}:{tag}"),
    );

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

    match env::var(COSIGN_PRIVATE_KEY).ok() {
        Some(cosign_priv_key) if !cosign_priv_key.is_empty() && Path::new(COSIGN_PATH).exists() => {
            env::set_var("COSIGN_PASSWORD", "");
            env::set_var("COSIGN_YES", "true");

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
            let found_pub_key = fs::read_to_string(COSIGN_PATH)?;
            trace!("calculated_pub_key={calculated_pub_key},found_pub_key={found_pub_key}");

            if calculated_pub_key.trim() == found_pub_key.trim() {
                debug!("Cosign files match, continuing build");
                Ok(())
            } else {
                bail!("Public key '{COSIGN_PATH}' does not match private key")
            }
        }
        _ => {
            warn!("{COSIGN_PATH} doesn't exist, skipping cosign file check");
            Ok(())
        }
    }
}
