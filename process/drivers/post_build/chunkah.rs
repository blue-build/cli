use std::num::NonZeroU32;

use blue_build_utils::{
    constants::DEFAULT_MAX_LAYERS,
    container::{ContainerId, ImageRef, OciRef},
    tempdir,
};
use bon::Builder;
use comlexr::cmd;
use log::trace;
use miette::{IntoDiagnostic, Result, bail};
use oci_client::Reference;

use crate::{
    drivers::{
        BuildDriver, BuildDriverType, Driver, ImageStorageDriver, OciCopy, PodmanDriver, PostBuild,
        PostBuildRunner,
        opts::{CopyOciOpts, InspectImageOpts, PostBuildOpts, PullOpts},
    },
    logging::CommandLogging,
    signal_handler::{ContainerRuntime, ContainerSignalId, add_cid, remove_cid},
};

#[derive(Debug, Copy, Clone, Builder)]
#[builder(derive(Debug, Clone))]
pub struct Chunkah<'a> {
    /// Maximum number of layers to use.
    #[builder(default = DEFAULT_MAX_LAYERS)]
    pub max_layers: NonZeroU32,

    /// Chunkah tag to use.
    #[builder(default = "latest")]
    pub tag: &'a str,

    /// Digest of Chunkah image to use.
    pub digest: Option<&'a str>,
}

impl Chunkah<'_> {
    pub const REGISTRY: &'static str = "quay.io";
    pub const REPOSITORY: &'static str = "coreos/chunkah";
}

impl PostBuild for Chunkah<'_> {
    fn check_driver_requirements(&self) -> Result<()> {
        trace!("Chunkah::check_driver_requirements({self:#?})");
        if !matches!(Driver::get_build_driver(), BuildDriverType::Podman) {
            bail!("Chunkah requires podman to be used as the build driver");
        }
        Ok(())
    }

    fn init(&self) -> Result<Box<dyn PostBuildRunner>> {
        trace!("Chunkah::init({self:#?})");
        let registry = Self::REGISTRY.to_owned();
        let repository = Self::REPOSITORY.to_owned();
        let chunkah_image_ref = if let Some(digest) = self.digest {
            let digest = digest.to_owned();
            Reference::with_digest(registry, repository, digest)
        } else {
            let tag = self.tag.to_owned();
            Reference::with_tag(registry, repository, tag)
        };
        let chunkah_image_id = PodmanDriver::pull(
            PullOpts::builder()
                .image(&chunkah_image_ref)
                .retry_count(5)
                .build(),
        )?;
        Ok(Box::new(ChunkahRunner {
            max_layers: self.max_layers,
            chunkah_image_id,
        }))
    }
}

#[derive(Debug, Clone)]
pub struct ChunkahRunner {
    /// Maximum number of layers to use.
    pub max_layers: NonZeroU32,

    /// Image ID of Chunkah image
    pub chunkah_image_id: ContainerId,
}

impl PostBuildRunner for ChunkahRunner {
    fn post_build(&self, opts: PostBuildOpts) -> Result<()> {
        trace!("ChunkahRunner::post_build({self:#?}, {opts:#?})");

        let chunkah_temp_dir = tempdir()?;
        let config_path = chunkah_temp_dir.path().join("chunkah_config.json");
        let oci_dir = chunkah_temp_dir.path().join("oci-out");

        PodmanDriver::inspect_image(
            InspectImageOpts::builder()
                .image(&opts.input_image.to_string())
                .output_path(&config_path)
                .build(),
        )?;

        let output_image_str = opts.output_image.to_string();

        let cid_dir = tempdir()?;
        let cid_path = cid_dir.path().join("cid");
        let cid = ContainerSignalId::new(&cid_path, ContainerRuntime::Podman, false);
        add_cid(&cid);

        let chunkah_status = {
            let c = cmd!(
                "podman",
                "run",
                "--cidfile",
                cid_path,
                "--pull=never",
                "--rm",
                format!(
                    "--mount=type=image,src={},destination=/chunkah",
                    opts.input_image
                ),
                "--volume",
                format!("{path}:{path}:Z", path = chunkah_temp_dir.path().display()),
                "--",
                &self.chunkah_image_id,
                "build",
                "--compressed",
                "--config",
                config_path,
                "--prune",
                "/sysroot/",
                "--max-layers",
                self.max_layers.to_string(),
                "--label",
                "ostree.commit-",
                "--label",
                "ostree.final-diffid-",
                "--tag",
                &output_image_str,
                "--output",
                format!("oci:{}", oci_dir.display()),
            );
            trace!("{c:?}");
            c
        }
        .build_status(
            &output_image_str,
            format!("Running Chunkah on image {output_image_str}"),
        )
        .into_diagnostic()?;

        if !chunkah_status.success() {
            bail!("Chunkah child process failed with exit code {chunkah_status}");
        }

        remove_cid(&cid);

        let dest_ref = match opts.output_image.clone() {
            ImageRef::Remote(image_ref) => OciRef::LocalStorage(image_ref.into_owned()),
            ImageRef::LocalTar(path) => OciRef::OciArchive(path.into_owned()),
            ImageRef::Other(other) => bail!("Unknown image ref type: {other}"),
        };

        Driver.copy_oci(
            CopyOciOpts::builder()
                .src_ref(&OciRef::OciDir(oci_dir))
                .dest_ref(&dest_ref)
                .podman_unshare(true)
                .build(),
        )?;

        Ok(())
    }
}
