use std::{
    borrow::Cow,
    ops::Deref,
    path::{Path, PathBuf},
};

use oci_distribution::Reference;

#[derive(Debug, Clone)]
pub struct ContainerId(pub(crate) String);

impl Deref for ContainerId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for ContainerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl AsRef<std::ffi::OsStr> for ContainerId {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.0.as_ref()
    }
}

pub struct MountId(pub(crate) String);

impl Deref for MountId {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for MountId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl AsRef<std::ffi::OsStr> for MountId {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.0.as_ref()
    }
}

impl<'a> From<&'a MountId> for std::borrow::Cow<'a, str> {
    fn from(value: &'a MountId) -> Self {
        Self::Borrowed(&value.0)
    }
}

#[derive(Debug, Clone)]
pub struct OciDir(String);

impl std::fmt::Display for OciDir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl AsRef<std::ffi::OsStr> for OciDir {
    fn as_ref(&self) -> &std::ffi::OsStr {
        self.0.as_ref()
    }
}

impl TryFrom<std::path::PathBuf> for OciDir {
    type Error = miette::Report;

    fn try_from(value: std::path::PathBuf) -> Result<Self, Self::Error> {
        if !value.is_dir() {
            miette::bail!("OCI directory doesn't exist at {}", value.display());
        }

        Ok(Self(format!("oci:{}", value.display())))
    }
}

/// An image ref that could reference
/// a remote registry or a local tarball.
#[derive(Debug, Clone)]
pub enum ImageRef<'scope> {
    Remote(Cow<'scope, Reference>),
    LocalTar(Cow<'scope, Path>),
    Other(Cow<'scope, str>),
}

impl ImageRef<'_> {
    #[must_use]
    pub fn remote_ref(&self) -> Option<&Reference> {
        match self {
            Self::Remote(remote) => Some(remote.as_ref()),
            _ => None,
        }
    }
}

impl<'scope> From<&'scope Self> for ImageRef<'scope> {
    fn from(value: &'scope ImageRef) -> Self {
        match value {
            Self::Remote(remote) => Self::Remote(Cow::Borrowed(remote.as_ref())),
            Self::LocalTar(path) => Self::LocalTar(Cow::Borrowed(path.as_ref())),
            Self::Other(other) => Self::Other(Cow::Borrowed(other.as_ref())),
        }
    }
}

impl<'scope> From<&'scope Reference> for ImageRef<'scope> {
    fn from(value: &'scope Reference) -> Self {
        Self::Remote(Cow::Borrowed(value))
    }
}

impl From<Reference> for ImageRef<'_> {
    fn from(value: Reference) -> Self {
        Self::Remote(Cow::Owned(value))
    }
}

impl<'scope> From<&'scope Path> for ImageRef<'scope> {
    fn from(value: &'scope Path) -> Self {
        Self::LocalTar(Cow::Borrowed(value))
    }
}

impl<'scope> From<&'scope PathBuf> for ImageRef<'scope> {
    fn from(value: &'scope PathBuf) -> Self {
        Self::from(value.as_path())
    }
}

impl From<PathBuf> for ImageRef<'_> {
    fn from(value: PathBuf) -> Self {
        Self::LocalTar(Cow::Owned(value))
    }
}

impl From<ImageRef<'_>> for String {
    fn from(value: ImageRef<'_>) -> Self {
        Self::from(&value)
    }
}

impl From<&ImageRef<'_>> for String {
    fn from(value: &ImageRef<'_>) -> Self {
        format!("{value}")
    }
}

impl std::fmt::Display for ImageRef<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Remote(remote) => remote.whole(),
                Self::LocalTar(path) => format!("oci-archive:{}", path.display()),
                Self::Other(other) => other.to_string(),
            }
        )
    }
}

impl PartialEq<Reference> for ImageRef<'_> {
    fn eq(&self, other: &Reference) -> bool {
        match self {
            Self::Remote(remote) => &**remote == other,
            _ => false,
        }
    }
}
