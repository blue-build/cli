use std::path::PathBuf;

use blue_build_utils::{container::Tag, string_vec};
use comlexr::cmd;
use log::trace;
use miette::{Result, bail};

use super::{CiDriver, Driver, opts::GenerateTagsOpts};

pub struct LocalDriver;

impl CiDriver for LocalDriver {
    fn on_default_branch() -> bool {
        trace!("LocalDriver::on_default_branch()");
        false
    }

    fn keyless_cert_identity() -> Result<String> {
        bail!("Unimplemented for local")
    }

    fn oidc_provider() -> miette::Result<String> {
        bail!("Unimplemented for local")
    }

    fn generate_tags(opts: GenerateTagsOpts) -> Result<Vec<Tag>> {
        trace!("LocalDriver::generate_tags({opts:?})");
        let os_version = Driver::get_os_version().oci_ref(opts.oci_ref).call()?;
        let timestamp = blue_build_utils::get_tag_timestamp();
        let short_sha = commit_sha();

        opts.alt_tags
            .as_ref()
            .map_or_else(
                || {
                    let mut tags = string_vec![
                        "latest",
                        &timestamp,
                        format!("{os_version}"),
                        format!("{timestamp}-{os_version}"),
                    ];

                    if let Some(short_sha) = &short_sha {
                        tags.push(format!("{short_sha}-{os_version}"));
                    }

                    tags
                },
                |alt_tags| {
                    alt_tags
                        .iter()
                        .flat_map(|alt| {
                            let mut tags = string_vec![
                                alt,
                                format!("{alt}-{os_version}"),
                                format!("{timestamp}-{alt}-{os_version}"),
                            ];
                            if let Some(short_sha) = &short_sha {
                                tags.push(format!("{short_sha}-{alt}-{os_version}"));
                            }

                            tags
                        })
                        .collect()
                },
            )
            .into_iter()
            .map(|tag| tag.parse())
            .collect()
    }

    fn get_repo_url() -> miette::Result<String> {
        trace!("LocalDriver::get_repo_url()");
        Ok(String::new())
    }

    fn get_registry() -> miette::Result<String> {
        trace!("LocalDriver::get_registry()");
        Ok(String::from("localhost"))
    }

    fn default_ci_file_path() -> PathBuf {
        unimplemented!()
    }
}

fn commit_sha() -> Option<String> {
    let output = cmd!("git", "rev-parse", "--short", "HEAD").output().ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
    } else {
        None
    }
}
