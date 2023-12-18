use std::{env, fs, path::Path, process::Command};

use anyhow::{anyhow, bail, Result};
use chrono::Local;

use crate::module_recipe::Recipe;

fn check_command_exists(command: &str) -> Result<()> {
    eprintln!("Checking if {command} exists...");
    match Command::new("command")
        .arg("-v")
        .arg(command)
        .status()?
        .success()
    {
        true => {
            eprintln!("Command {command} does exist");
            Ok(())
        }
        false => Err(anyhow!(
            "Command {command} doesn't exist and is required to build the image"
        )),
    }
}

fn generate_tags(recipe: &Recipe) -> Vec<String> {
    eprintln!("Generating image tags for {}", &recipe.name);

    let mut tags: Vec<String> = Vec::new();
    let image_version = recipe.image_version;
    let timestamp = Local::now().format("%Y%m%d").to_string();

    if env::var("CI").is_ok() {
        eprintln!("Detected running in Gitlab, pulling information from CI variables...");

        if let (Ok(mr_iid), Ok(pipeline_source)) = (
            env::var("CI_MERGE_REQUEST_IID"),
            env::var("CI_PIPELINE_SOURCE"),
        ) {
            if pipeline_source == "merge_request_event" {
                eprintln!("Running in a MR...");
                tags.push(format!("{mr_iid}-{image_version}"));
            }
        }

        if let Ok(commit_sha) = env::var("CI_COMMIT_SHORT_SHA") {
            tags.push(format!("{commit_sha}-{image_version}"));
        }

        if let (Ok(commit_branch), Ok(default_branch)) =
            (env::var("CI_COMMIT_BRANCH"), env::var("CI_DEFAULT_BRANCH"))
        {
            if default_branch != commit_branch {
                eprintln!("Running on branch {commit_branch}...");
                tags.push(format!("br-{commit_branch}-{image_version}"));
            } else {
                eprintln!("Running on the default branch...");
                tags.push(image_version.to_string());
                tags.push(format!("{image_version}-{timestamp}"));
                tags.push(timestamp.to_string());
            }
        }
    } else {
        eprintln!("Running locally...");
        tags.push(format!("{image_version}-local"));
    }
    eprintln!("Finished generating tags!");
    eprintln!("Tags: {tags:?}");
    tags
}

fn login(
    registry: Option<&String>,
    username: Option<&String>,
    password: Option<&String>,
) -> Result<()> {
    eprintln!("Attempting to login to the registry");
    let registry = match registry {
        Some(registry) => registry.to_owned(),
        None => env::var("CI_REGISTRY")?,
    };

    let username = match username {
        Some(username) => username.to_owned(),
        None => env::var("CI_REGISTRY_USER")?,
    };

    let password = match password {
        Some(password) => password.to_owned(),
        None => env::var("CI_REGISTRY_PASSWORD")?,
    };

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
        true => eprintln!("Buildah login success at {registry} for user {username}!"),
        false => return Err(anyhow!("Failed to login for buildah!")),
    }

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
        true => eprintln!("Cosign login success at {registry} for user {username}!"),
        false => return Err(anyhow!("Failed to login for cosign!")),
    }

    Ok(())
}

fn generate_full_image_name(
    recipe: &Recipe,
    registry: Option<&String>,
    registry_path: Option<&String>,
    push: bool,
) -> Result<String> {
    eprintln!("Generating full image name");
    let image_name = recipe.name.as_str();

    let image_name = if env::var("CI").is_ok() {
        eprintln!("Detected running in Gitlab CI...");
        if let (Ok(registry), Ok(project_namespace), Ok(project_name)) = (
            env::var("CI_REGISTRY"),
            env::var("CI_PROJECT_NAMESPACE"),
            env::var("CI_PROJECT_NAME"),
        ) {
            format!("{registry}/{project_namespace}/{project_name}/{image_name}")
        } else {
            bail!("Unable to generate image name for Gitlab CI env!")
        }
    } else {
        eprintln!("Detected running locally...");
        if let (Some(registry), Some(registry_path)) = (registry, registry_path) {
            format!(
                "{}/{}/{image_name}",
                registry.trim_matches('/'),
                registry_path.trim_matches('/')
            )
        } else {
            if push {
                bail!("Need '--registry' and '--registry-path' in order to push image");
            }
            image_name.to_string()
        }
    };

    eprintln!("Using image name {image_name}");

    Ok(image_name)
}

fn build(image_name: &str, tags: &[String], push: bool) -> Result<()> {
    let mut tags_iter = tags.iter();

    let first_tag = tags_iter
        .next()
        .ok_or(anyhow!("We got here with no tags!?"))?;

    let mut build = Command::new("buildah")
        .arg("build")
        .arg("-t")
        .arg(format!("{image_name}:{first_tag}"))
        .spawn()?;

    let status = build.wait()?;

    if status.success() {
        eprintln!("Successfully built {image_name}");
    } else {
        bail!("Failed to build {image_name}");
    }

    if tags.len() > 1 {
        eprintln!("Tagging all images...");
        for tag in tags_iter {
            eprintln!("Tagging {image_name} with {tag}");
            let mut child = Command::new("buildah")
                .arg("tag")
                .arg(format!("{image_name}:{first_tag}"))
                .arg(format!("{image_name}:{tag}"))
                .spawn()?;

            if child.wait()?.success() {
                eprintln!("Successfully tagged {image_name}:{tag}!");
            } else {
                bail!("Failed to tag image {image_name}:{tag}");
            }
        }
    }

    if push {
        eprintln!("Pushing all images...");
        for tag in tags.iter() {
            eprintln!("Pushing image {image_name}:{tag}...");
            let mut child = Command::new("buildah")
                .arg("push")
                .arg(format!("{image_name}:{tag}"))
                .spawn()?;

            if child.wait()?.success() {
                eprintln!("Successfully pushed {image_name}:{tag}!")
            } else {
                bail!("Failed to push image {image_name}:{tag}");
            }
        }

        sign_images(image_name, first_tag)?;
    }

    Ok(())
}

fn sign_images(image_name: &str, tag: &str) -> Result<()> {
    if env::var("SIGSTORE_ID_TOKEN").is_ok() && env::var("CI").is_ok() {
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
            if default_branch == commit_branch {
                eprintln!("Retrieving image digest...");
                let image_digest = String::from_utf8(
                    Command::new("skopeo")
                        .arg("inspect")
                        .arg("--format='{{.Digest}}'")
                        .arg(format!("docker://{image_name}:{tag}"))
                        .output()?
                        .stdout,
                )?;

                eprintln!("Signing image: {image_name}@{image_digest}");

                let mut child = Command::new("cosign")
                    .arg("sign")
                    .arg(format!("{image_name}@{image_digest}"))
                    .spawn()?;

                if child.wait()?.success() {
                    eprintln!("Successfully signed image!");
                } else {
                    bail!("Failed to sign image: {image_name}@{image_digest}");
                }

                let mut child = Command::new("cosign")
                    .arg("verify")
                    .arg("--certificate-identity")
                    .arg(format!(
                        "{project_url}//.gitlab-ci.yml@refs/heads/{default_branch}"
                    ))
                    .arg("--certificate-oidc-issuer")
                    .arg(format!("{server_protocol}://{server_host}"))
                    .arg(format!("{image_name}:{tag}"))
                    .spawn()?;

                if !child.wait()?.success() {
                    eprintln!("Failed to verify image!");
                }
            }
        }
    } else {
        eprintln!("No SIGSTORE_ID_TOKEN detected, not signing image");
    }

    Ok(())
}

pub fn build_image(
    recipe: &Path,
    registry: Option<&String>,
    registry_path: Option<&String>,
    username: Option<&String>,
    password: Option<&String>,
    push: bool,
) -> Result<()> {
    check_command_exists("buildah")?;
    if push {
        check_command_exists("cosign")?;
        check_command_exists("skopeo")?;
    }

    let recipe: Recipe = serde_yaml::from_str(fs::read_to_string(recipe)?.as_str())?;

    let tags = generate_tags(&recipe);

    let image_name = generate_full_image_name(&recipe, registry, registry_path, push)?;

    if push {
        login(registry, username, password)?;
    }
    build(&image_name, &tags, push)?;

    eprintln!("Build complete!");

    Ok(())
}
