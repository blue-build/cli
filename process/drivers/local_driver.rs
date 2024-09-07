use blue_build_utils::string_vec;
use log::trace;
use miette::bail;

use super::{opts::GenerateTagsOpts, CiDriver, Driver};

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

    fn generate_tags(opts: &GenerateTagsOpts) -> miette::Result<Vec<String>> {
        trace!("LocalDriver::generate_tags({opts:?})");
        let os_version = Driver::get_os_version(opts.oci_ref)?;
        Ok(opts.alt_tags.as_ref().map_or_else(
            || string_vec![format!("local-{os_version}")],
            |alt_tags| {
                alt_tags
                    .iter()
                    .flat_map(|alt| string_vec![format!("local-{alt}-{os_version}")])
                    .collect()
            },
        ))
    }

    fn get_repo_url() -> miette::Result<String> {
        trace!("LocalDriver::get_repo_url()");
        Ok(String::new())
    }

    fn get_registry() -> miette::Result<String> {
        trace!("LocalDriver::get_registry()");
        Ok(String::from("localhost"))
    }
}
