use std::{fs, iter::once};

use blue_build_utils::tempdir;
use cached::once;
use comlexr::cmd;
use miette::{IntoDiagnostic, Result};

use crate::layer::{FinalizeLayer, FromLayer, Layer, LayerCommand, LayerId, UnshareLayer};

include!(concat!(env!("OUT_DIR"), "/build_scripts.rs"));

/// This produces the layer containing the embeded
/// build scripts that are used as a harness for
/// executing the modules.
#[once]
pub fn build_scripts_layer() -> Result<LayerId> {
    let temp_dir = tempdir()?;
    let files = BUILD_SCRIPTS
        .iter()
        .map(|(file, contents)| {
            let path = temp_dir.path().join(file);
            fs::write(&path, contents).into_diagnostic()?;
            Ok((path, file))
        })
        .collect::<Result<Vec<_>>>()?;
    let from = FromLayer::builder().build();
    Layer::from(
        UnshareLayer::builder()
            .parent(from)
            .commands(
                once(LayerCommand::from(cmd!(
                    "mkdir",
                    "-p",
                    "${BB_UNSHARE_MOUNT}/scripts"
                )))
                .chain(
                    files
                        .iter()
                        .flat_map(|(src, dest)| {
                            let dest = format!("${{BB_UNSHARE_MOUNT}}/scripts/{dest}");
                            [cmd!("cp", "-f", src, &dest), cmd!("chmod", "+x", dest)]
                        })
                        .map(LayerCommand::from),
                ),
            )
            .build(),
    )
    .finalize()
}
