use std::{fs, path::PathBuf};

use anyhow::{bail, Result};
use clap::Args;
use typed_builder::TypedBuilder;

use crate::drivers::{
    opts::{RunOpts, RunOptsVolume},
    Driver,
};

use super::{BlueBuildCommand, DriverArgs};

#[derive(Default, Clone, Debug, TypedBuilder, Args)]
pub struct GenerateIsoCommand {
    #[arg(long)]
    image_tar: Option<String>,

    #[arg(short = 'r', long)]
    image_repo: Option<String>,

    #[arg(short = 'n', long)]
    image_name: String,

    #[arg(short = 't', long)]
    image_tag: String,

    #[arg(short = 'V', long)]
    variant: String,

    #[arg(long)]
    iso_name: Option<String>,

    #[arg(short, long)]
    output_dir: PathBuf,

    #[clap(flatten)]
    #[builder(default)]
    drivers: DriverArgs,
}

impl BlueBuildCommand for GenerateIsoCommand {
    fn try_run(&mut self) -> Result<()> {
        Driver::builder()
            .build_driver(self.drivers.build_driver)
            .inspect_driver(self.drivers.inspect_driver)
            .run_driver(self.drivers.run_driver)
            .build()
            .init();

        if self.output_dir.exists() && !self.output_dir.is_dir() {
            bail!("The '--output-dir' arg must be a directory");
        }

        let iso_name = self
            .iso_name
            .as_ref()
            .map_or_else(|| "build.iso", |name| name.as_str());
        let iso_path = self.output_dir.join(iso_name);

        if !self.output_dir.exists() {
            fs::create_dir(&self.output_dir)?;
        }

        if iso_path.exists() {
            fs::remove_file(iso_path)?;
        }

        let run_driver = Driver::get_run_driver();

        let volumes = [
            RunOptsVolume::builder()
                .path_or_vol_name(self.output_dir.display().to_string())
                .container_path("/build-container-installer/build")
                .build(),
            RunOptsVolume::builder()
                .path_or_vol_name("dnf-cache")
                .container_path("/cache/dnf")
                .build(),
            // RunOptsVolume::builder()
            //     .path_or_vol_name(&self.image_tar)
            //     .container_path("/image.tar.gz")
            //     .build(),
        ];

        let args = [
            format!("IMAGE_NAME={}", self.image_name),
            format!("IMAGE_TAG={}", self.image_tag),
            format!("VARIANT={}", self.variant),
            format!("ISO_NAME={iso_name}"),
            "DNF_CACHE=/cache/dnf".to_string(),
        ];

        // Currently testing local tarball builds
        let opts = RunOpts::builder()
            .image("ghcr.io/jasonn3/build-container-installer")
            .privileged(true)
            .remove(true)
            .args(&args)
            .volumes(&volumes)
            .build();

        let status = run_driver.run(&opts)?;

        if !status.success() {
            bail!("Failed to create ISO");
        }
        Ok(())
    }
}
