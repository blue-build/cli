use std::{fmt::Display, path::PathBuf, sync::Arc};

use lazy_regex::regex;
use miette::{Result, bail};

use crate::{
    layer::{FinalizeLayer, Layer, LayerId},
    path::ContextPath,
};

/// A `Mount` for a layer. Typically used in `RunLayer`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Mount {
    typ: Arc<MountType>,
    common: Arc<MountCommon>,
}

#[bon::bon]
impl Mount {
    /// Builder for `Mount`.
    #[builder(finish_fn = "build", derive(Into))]
    pub fn new_bind(
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
            typ: Arc::new(MountType::Bind { source, selinux }),
            common: Arc::new(MountCommon { destination, mode }),
        }
    }

    #[builder(finish_fn = "build", derive(Into))]
    pub fn new_cache(
        /// The global `CacheId` for the mount.
        ///
        /// # Errors
        /// Will fail if the cache ID has invalid characters.
        #[builder(with = |id: &str| -> miette::Result<_> {
            CacheId::try_from(id)
        })]
        id: CacheId,

        /// Source `PathBuf` in the cache mount.
        #[builder(into)]
        source: Option<PathBuf>,

        /// The destination in the build to mount to.
        #[builder(into)]
        destination: PathBuf,

        /// The access mode of the mount.
        #[builder(default)]
        mode: MountMode,
    ) -> Self {
        Self {
            typ: Arc::new(MountType::Cache { id, source }),
            common: Arc::new(MountCommon { destination, mode }),
        }
    }
}

impl Mount {
    pub(super) fn finalize(&self) -> Result<String> {
        Ok(format!(
            "--mount=type={mount}",
            mount = match &*self.typ {
                MountType::Bind { source, selinux } => {
                    format!(
                        "bind{source}{common}{selinux}",
                        source = source.finalize()?,
                        common = self.common
                    )
                }
                MountType::Cache { id, source } => format!(
                    "cache,id={id}{src}{common}",
                    src = source
                        .as_ref()
                        .map_or_default(|source| source.display().to_string()),
                    common = self.common
                ),
            }
        ))
    }
}

/// The type of the `Mount`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum MountType {
    /// Bind mount.
    Bind {
        /// The source of the mount.
        source: MountSource,

        /// The SELinux mode of the mount.
        selinux: SeLinuxMode,
    },
    /// Cache mount.
    Cache {
        /// the ID of the cache.
        id: CacheId,

        /// The source path in the cache mount.
        source: Option<PathBuf>,
    },
}

/// An ID for cache `Mount`s.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheId(String);

impl TryFrom<&str> for CacheId {
    type Error = miette::Error;

    fn try_from(value: &str) -> std::prelude::v1::Result<Self, Self::Error> {
        if regex!("[a-zA-Z0-9_-]+").is_match(value) {
            Ok(Self(value.to_string()))
        } else {
            bail!("String {value} a valid CacheId")
        }
    }
}

impl std::fmt::Display for CacheId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// SELinux mode for the mount.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum SeLinuxMode {
    /// Blank label.
    #[default]
    Default,

    /// Labels the mount for shared access.
    ///
    /// Equivalent to `"z"`.
    Shared,

    /// Labels the mount for private access.
    ///
    /// Equivalent to `"Z"`.
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

/// Struct containing common `Mount` options.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MountCommon {
    /// The destination in the container to mount.
    destination: PathBuf,

    /// The mode the mount is set to.
    mode: MountMode,
}

impl Display for MountCommon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, ",dst={}{}", self.destination.display(), self.mode)
    }
}

/// The access mode of the `Mount`.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub enum MountMode {
    /// The default mode for the mount.
    #[default]
    Default,

    /// Set access for the mount to read only.
    ///
    /// Equivalent to `"ro"`.
    ReadOnly,

    /// Set access for the mount to read write.
    ///
    /// Equivalent to `"rw"`.
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

/// The source for the mount.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MountSource {
    /// A `Layer` source with a `PathBuf`.
    Layer(Layer, PathBuf),

    /// A completed `Layer` in the form of a `LayerId`.
    CompleteLayer(LayerId, PathBuf),

    /// A `ContextPath` mount.
    Path(ContextPath),
}

impl<L: Into<Layer>, P: Into<PathBuf>> From<(L, P)> for MountSource {
    fn from((layer, path): (L, P)) -> Self {
        Self::Layer(layer.into(), path.into())
    }
}

impl<P: Into<PathBuf>> From<(LayerId, P)> for MountSource {
    fn from((layer, path): (LayerId, P)) -> Self {
        Self::CompleteLayer(layer, path.into())
    }
}

impl From<ContextPath> for MountSource {
    fn from(value: ContextPath) -> Self {
        Self::Path(value)
    }
}

impl MountSource {
    fn finalize(&self) -> Result<String> {
        Ok(match self {
            Self::Layer(layer, src) => format!(
                ",from={layer},src={src}",
                src = src.display(),
                layer = layer.clone().finalize()?
            ),
            Self::CompleteLayer(layer, src) => {
                format!(",from={layer},src={src}", src = src.display())
            }
            Self::Path(path) => format!(",src={}", path.path().display()),
        })
    }
}
