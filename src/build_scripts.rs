use std::{
    fs::{self, DirEntry, OpenOptions},
    io::{Read, Write},
    ops::Not,
    os::unix::fs::PermissionsExt,
    path::PathBuf,
};

use blue_build_utils::constants::{BLUE_BUILD_SCRIPTS_DIR_IGNORE, GITIGNORE_PATH};
use miette::{Context, IntoDiagnostic, Result, miette};
use rust_embed::Embed;

const SCRIPT_DIR_PREFIX: &str = ".bluebuild-scripts_";

#[derive(Embed)]
#[folder = "scripts/"]
pub struct BuildScripts;

impl BuildScripts {
    /// Extracts the build scripts into the build directory.
    /// This will also remove any old build scripts that
    /// were not cleaned up previously.
    ///
    /// # Errors
    /// Will error if the scripts cannot be extracted or the
    /// old scripts cannot be deleted.
    pub fn extract_mount_dir() -> Result<PathBuf> {
        update_gitignore()?;
        delete_old_dirs()?;

        let dir = PathBuf::from(format!(
            "{SCRIPT_DIR_PREFIX}{}",
            crate::shadow::SHORT_COMMIT
        ));
        fs::create_dir(&dir)
            .into_diagnostic()
            .wrap_err("Failed to create dir for build scripts.")?;

        for file_path in Self::iter() {
            let file = Self::get(file_path.as_ref())
                .ok_or_else(|| miette!("Failed to get file {file_path}"))?;
            let file_path = dir.join(&*file_path);
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

        Ok(dir)
    }
}

fn delete_old_dirs() -> Result<()> {
    let dirs = fs::read_dir(".")
        .into_diagnostic()
        .wrap_err("Failed to read current directory")?
        .collect::<std::result::Result<Vec<DirEntry>, _>>()
        .into_diagnostic()
        .wrap_err("Failed to read dir entry")?;

    for dir in dirs {
        if dir
            .file_name()
            .display()
            .to_string()
            .starts_with(SCRIPT_DIR_PREFIX)
        {
            fs::remove_dir_all(dir.path())
                .into_diagnostic()
                .wrap_err_with(|| {
                    format!(
                        "Failed to remove old build script dir at {}",
                        dir.path().display()
                    )
                })?;
        }
    }

    Ok(())
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
