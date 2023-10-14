const GITLAB_CI_FILE: &'static str = include_str!("../templates/init/gitlab-ci.yml");
const RECIPE_FILE: &'static str = include_str!("../templates/init/recipe.yml");
const LICENSE_FILE: &'static str = include_str!("../LICENSE");

pub fn initialize_directory(base_dir: PathBuf) {
    let recipe_path = base_dir.join("recipe.yml");

    let gitlab_ci_path = base_dir.join(".gitlab-ci.yml");

    let readme_path = base_dir.join("README.md");

    let license_path = base_dir.join("LICENSE");

    let scripts_dir = base_dir.join("scripts/");

    let pre_scripts_dir = scripts_dir.join("pre/");

    let post_scripts_dir = scripts_dir.join("post/");
}
