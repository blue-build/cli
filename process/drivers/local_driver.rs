use log::trace;
use miette::bail;

use super::{CiDriver, Driver};

pub struct LocalDriver;

impl CiDriver for LocalDriver {
    fn on_default_branch() -> bool {
        trace!("LocalDriver::on_default_branch()");
        false
    }

    fn keyless_cert_identity() -> miette::Result<String> {
        trace!("LocalDriver::keyless_cert_identity()");
        bail!("Keyless not supported");
    }

    fn oidc_provider() -> miette::Result<String> {
        trace!("LocalDriver::oidc_provider()");
        bail!("Keyless not supported");
    }

    fn generate_tags(recipe: &blue_build_recipe::Recipe) -> miette::Result<Vec<String>> {
        trace!("LocalDriver::generate_tags({recipe:?})");
        Ok(vec![format!("local-{}", Driver::get_os_version(recipe)?)])
    }

    fn generate_image_name(recipe: &blue_build_recipe::Recipe) -> miette::Result<String> {
        trace!("LocalDriver::generate_image_name({recipe:?})");
        Ok(recipe.name.trim().to_lowercase())
    }

    fn get_repo_url() -> miette::Result<String> {
        trace!("LocalDriver::get_repo_url()");
        Ok(String::new())
    }

    fn get_registry() -> miette::Result<String> {
        trace!("LocalDriver::get_registry()");
        Ok(String::new())
    }
}
