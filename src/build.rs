use std::{env, fs, path::Path, process::Command};

use anyhow::{anyhow, Result};
use chrono::{Datelike, Local};

use crate::module_recipe::Recipe;

fn check_command_exists(command: &str) -> Result<()> {
    match Command::new("command")
        .arg("-v")
        .arg(command)
        .status()?
        .success()
    {
        true => Ok(()),
        false => Err(anyhow!(
            "Command {command} doesn't exist and is required to build the image"
        )),
    }
}

fn generate_tags(recipe: &Recipe) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    let image_version = recipe.image_version;
    let timestamp = Local::now().format("%Y%m%d").to_string();

    if let Ok(_) = env::var("CI") {
        if let (Ok(mr_iid), Ok(pipeline_source)) = (
            env::var("CI_MERGE_REQUEST_IID"),
            env::var("CI_PIPELINE_SOURCE"),
        ) {
            if pipeline_source == "merge_request_event" {
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
                tags.push(format!("br-{commit_branch}-{image_version}"));
            } else {
                tags.push(format!("{image_version}"));
                tags.push(format!("{image_version}-{timestamp}"));
                tags.push(format!("{timestamp}"));
            }
        }
    } else {
        tags.push(format!("{image_version}-local"));
    }
    tags
}

fn login(
    registry: Option<&String>,
    username: Option<&String>,
    password: Option<&String>,
) -> Result<()> {
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
        true => eprintln!("Buildah login success!"),
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
        true => eprintln!("Cosign login success!"),
        false => return Err(anyhow!("Failed to login for cosign!")),
    }

    Ok(())
}

fn generate_full_image_name(
    recipe: &Recipe,
    registry: Option<&String>,
    registry_path: Option<&String>,
) -> Result<String> {
    let image_name = recipe.name.as_str();

    if let Ok(_) = env::var("CI") {
        // if let (Ok())
        todo!()
    } else {
        Ok(image_name.to_string())
    }
}

fn build(recipe: &Recipe, image_name: &str, tags: &[String]) -> Result<()> {
    todo!()
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
        login(registry.clone(), username.clone(), password.clone())?;
    }

    let recipe: Recipe = serde_yaml::from_str(fs::read_to_string(recipe)?.as_str())?;

    let tags = generate_tags(&recipe);

    let image_name = generate_full_image_name(&recipe, registry.clone(), registry_path.clone())?;

    build(&recipe, &image_name, &tags)?;

    Ok(())
}
