mod akmods_info;
mod maybe_version;
mod module;
mod module_ext;
mod recipe;
mod stage;
mod stages_ext;

use std::path::{Path, PathBuf};

use blue_build_utils::constants::{CONFIG_PATH, RECIPE_PATH};
use log::warn;

pub use akmods_info::*;
pub use maybe_version::*;
pub use module::*;
pub use module_ext::*;
pub use recipe::*;
pub use stage::*;
pub use stages_ext::*;

pub trait FromFileList {
    const LIST_KEY: &str;

    fn get_from_file_paths(&self) -> Vec<PathBuf>;

    fn get_module_from_file_paths(&self) -> Vec<PathBuf> {
        Vec::new()
    }
}

pub(crate) fn base_recipe_path() -> &'static Path {
    let legacy_path = Path::new(CONFIG_PATH);
    let recipe_path = Path::new(RECIPE_PATH);

    if recipe_path.exists() && recipe_path.is_dir() {
        recipe_path
    } else {
        warn!("Use of {CONFIG_PATH} for recipes is deprecated, please move your recipe files into {RECIPE_PATH}");
        legacy_path
    }
}
