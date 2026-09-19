use std::{fmt::Display, path::PathBuf, sync::Arc};

use miette::Result;

use crate::{
    layer::{FinalizeLayer, Layer},
    path::ContextPath,
    reference::PinnedReference,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Mount {
    typ: MountType,
    common: MountCommon,
}

#[bon::bon]
impl Mount {
    #[builder(finish_fn = "build")]
    pub const fn new_bind(
        /// The source of the bind mount.
        #[builder(into)]
        source: MountSource,

        /// The destination in the build to mount to.
        #[builder(into)]
        destination: PathBuf,

        /// The SELinux mode to set on the mount.
        #[builder(default)]
        selinux: SeLinuxMode,

        /// The access mode of the mount.
        #[builder(default)]
        mode: MountMode,
    ) -> Self {
        Self {
            typ: MountType::Bind { source, selinux },
            common: MountCommon { destination, mode },
        }
    }

    // #[builder(finish_fn = "build")]
    // pub const fn new_cache(
    //     id: CacheId,

    // )
}

impl Mount {
    pub(crate) fn finalize(&self) -> Result<String> {
        Ok(format!(
            "--mount=type={mount}",
            mount = match &self.typ {
                MountType::Bind { source, selinux } => {
                    format!(
                        "bind,{source}{common}{selinux}",
                        source = source.finalize()?,
                        common = self.common
                    )
                }
                MountType::Cache { id, source } => format!(
                    "cache,id={id},{src}{common}",
                    src = source
                        .as_ref()
                        .map_or_default(|source| source.display().to_string()),
                    common = self.common
                ),
            }
        ))
    }
}
// impl Display for Mount {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "--mount=type={value}")
//     }
// }

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MountType {
    Bind {
        source: MountSource,
        selinux: SeLinuxMode,
    },
    Cache {
        id: String,
        source: Option<PathBuf>,
    },
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum SeLinuxMode {
    #[default]
    Default,
    Shared,
    Private,
}

impl Display for SeLinuxMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Default => "",
                Self::Shared => ",z",
                Self::Private => ",Z",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MountCommon {
    pub destination: PathBuf,
    pub mode: MountMode,
}

impl Display for MountCommon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dst={}{}", self.destination.display(), self.mode)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum MountMode {
    #[default]
    Default,
    ReadOnly,
    ReadWrite,
}

impl Display for MountMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Default => "",
                Self::ReadOnly => ",ro",
                Self::ReadWrite => ",rw",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MountSource {
    Image(Arc<PinnedReference>, PathBuf),
    Layer(Layer, PathBuf),
    Path(ContextPath),
}

impl<P: Into<PathBuf>> From<(Arc<PinnedReference>, P)> for MountSource {
    fn from((image, path): (Arc<PinnedReference>, P)) -> Self {
        Self::Image(image, path.into())
    }
}

impl<L: Into<Layer>, P: Into<PathBuf>> From<(L, P)> for MountSource {
    fn from((layer, path): (L, P)) -> Self {
        Self::Layer(layer.into(), path.into())
    }
}

impl From<ContextPath> for MountSource {
    fn from(value: ContextPath) -> Self {
        Self::Path(value)
    }
}

// impl Display for MountSource {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         let value = match self {
//         };
//         write!(f, "{value}")
//     }
// }

impl MountSource {
    fn finalize(&self) -> Result<String> {
        Ok(match self {
            Self::Image(image, path) => format!("from={image},src={}", path.display()),
            Self::Layer(layer, src) => format!(
                "from={layer},src={src}",
                src = src.display(),
                layer = layer.clone().finalize()?
            ),
            Self::Path(path) => format!("src={}", path.path().display()),
        })
    }
}
