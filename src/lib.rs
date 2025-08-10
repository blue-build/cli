//! The root library for blue-build.
#![doc = include_str!("../README.md")]

use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    ops::Not,
    os::unix::fs::PermissionsExt,
};

use blue_build_process_management::drivers::types::BuildDriverType;
use blue_build_template::BuildEngine;
use blue_build_utils::constants::{BLUE_BUILD_SCRIPTS_DIR_IGNORE, GITIGNORE_PATH};
use miette::{Context, IntoDiagnostic, Result, miette};
use rust_embed::Embed;
use tempfile::TempDir;

pub mod commands;

shadow_rs::shadow!(shadow);

pub(crate) trait DriverTemplate {
    fn build_engine(&self) -> BuildEngine;
}

impl DriverTemplate for BuildDriverType {
    fn build_engine(&self) -> BuildEngine {
        match self {
            Self::Buildah | Self::Podman => BuildEngine::Oci,
            Self::Docker => BuildEngine::Docker,
        }
    }
}

#[derive(Embed)]
#[folder = "scripts/"]
pub(crate) struct BuildScripts;

impl BuildScripts {
    pub fn extract_mount_dir() -> Result<TempDir> {
        Self::update_gitignore()?;

        let tempdir = TempDir::with_prefix_in(".bluebuild-scripts_", ".")
            .into_diagnostic()
            .wrap_err("Failed to create tempdir for build scripts.")?;

        for file_path in Self::iter() {
            let file = Self::get(file_path.as_ref())
                .ok_or_else(|| miette!("Failed to get file {file_path}"))?;
            let file_path = tempdir.path().join(&*file_path);
            fs::write(&file_path, &file.data)
                .into_diagnostic()
                .wrap_err_with(|| {
                    format!("Failed to write build script file {}", file_path.display())
                })?;

            let mut perm = fs::metadata(&file_path)
                .into_diagnostic()
                .wrap_err_with(|| {
                    format!(
                        "Failed to get file permissions for file {}",
                        file_path.display()
                    )
                })?
                .permissions();

            perm.set_mode(0o755);
            fs::set_permissions(&file_path, perm).into_diagnostic()?;
        }

        Ok(tempdir)
    }

    fn update_gitignore() -> Result<()> {
        let file = &mut OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(GITIGNORE_PATH)
            .into_diagnostic()
            .wrap_err_with(|| format!("Failed to open {GITIGNORE_PATH} for editing"))?;

        let ignore_contents = {
            let mut cont = String::new();
            file.read_to_string(&mut cont)
                .into_diagnostic()
                .wrap_err_with(|| format!("Failed to read {GITIGNORE_PATH}"))?;
            cont
        };

        if ignore_contents
            .contains(BLUE_BUILD_SCRIPTS_DIR_IGNORE)
            .not()
        {
            writeln!(file, "{BLUE_BUILD_SCRIPTS_DIR_IGNORE}")
                .into_diagnostic()
                .wrap_err_with(|| {
                    format!("Failed to add {BLUE_BUILD_SCRIPTS_DIR_IGNORE} to {GITIGNORE_PATH}")
                })?;
        }

        Ok(())
    }
}
