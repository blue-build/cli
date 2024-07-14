use miette::bail;

use super::{CiDriver, Driver};

pub struct LocalDriver;

impl CiDriver for LocalDriver {
    fn on_default_branch() -> bool {
        false
    }

    fn keyless_cert_identity() -> miette::Result<String> {
        bail!("Keyless not supported");
    }

    fn oidc_provider() -> miette::Result<String> {
        bail!("Keyless not supported");
    }

    fn generate_tags(recipe: &blue_build_recipe::Recipe) -> miette::Result<Vec<String>> {
        Ok(vec![format!("local-{}", Driver::get_os_version(recipe)?)])
    }

    fn generate_image_name(recipe: &blue_build_recipe::Recipe) -> miette::Result<String> {
        Ok(recipe.name.trim().to_lowercase())
    }

    fn get_repo_url() -> miette::Result<String> {
        Ok(String::new())
    }

    fn get_registry() -> miette::Result<String> {
        Ok(String::new())
    }
}
