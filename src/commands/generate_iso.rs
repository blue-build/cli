use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::Args;
use typed_builder::TypedBuilder;

use crate::drivers::{
    opts::{RunOpts, RunOptsEnv, RunOptsVolume},
    Driver,
};

use super::{BlueBuildCommand, DriverArgs};

#[derive(Default, Clone, Debug, TypedBuilder, Args)]
pub struct GenerateIsoCommand {
    #[arg(short = 'r', long)]
    image_repo: String,

    #[arg(short = 'n', long)]
    image_name: String,

    #[arg(short = 't', long)]
    image_tag: String,

    #[arg(short = 'V', long)]
    variant: String,

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

        if !self.output_dir.exists() || !self.output_dir.is_dir() {
            bail!("The '--output-dir' arg must be a directory that exists");
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
        ];

        let envs = [
            RunOptsEnv::builder()
                .key("IMAGE_REPO")
                .value(&self.image_repo)
                .build(),
            RunOptsEnv::builder()
                .key("IMAGE_NAME")
                .value(&self.image_name)
                .build(),
            RunOptsEnv::builder()
                .key("IMAGE_TAG")
                .value(&self.image_tag)
                .build(),
            RunOptsEnv::builder()
                .key("VARIANT")
                .value(&self.variant)
                .build(),
            RunOptsEnv::builder()
                .key("DNF_CACHE")
                .value("/cache/dnf")
                .build(),
        ];

        let opts = RunOpts::builder()
            .image("ghcr.io/jasonn3/build-container-installer")
            .env_vars(&envs)
            .volumes(&volumes)
            .build();

        let status = run_driver.run(&opts)?;

        if !status.success() {
            bail!("Failed to create ISO");
        }
        Ok(())
    }
}
