use std::{
    borrow::Cow,
    ffi::OsString,
    ops::Deref,
    path::{Path, PathBuf},
    str::FromStr,
};

use lazy_regex::regex;
use miette::miette;
use oci_client::Reference;
use serde::{Deserialize, Serialize};

use crate::platform::Platform;

#[derive(Debug, Clone)]
pub struct ContainerId(pub String);

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

pub struct MountId(pub String);

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

#[derive(Clone, Debug)]
pub enum OciRef {
    LocalStorage(String),
    OciArchive(PathBuf),
    OciDir(PathBuf),
    Remote(Reference),
}

impl std::fmt::Display for OciRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LocalStorage(local_ref) => write!(f, "containers-storage:{local_ref}"),
            Self::OciArchive(path) => write!(f, "oci-archive:{}", path.display()),
            Self::OciDir(path) => write!(f, "oci:{}", path.display()),
            Self::Remote(image_ref) => write!(f, "docker://{}", image_ref.whole()),
        }
    }
}

impl From<Reference> for OciRef {
    fn from(image_ref: Reference) -> Self {
        Self::Remote(image_ref)
    }
}

impl From<&Reference> for OciRef {
    fn from(image_ref: &Reference) -> Self {
        Self::Remote(image_ref.clone())
    }
}

impl OciRef {
    #[must_use]
    pub fn from_local_storage(local_ref: &str) -> Self {
        Self::LocalStorage(local_ref.to_owned())
    }

    /// # Errors
    /// Returns an error if the path does not point to a regular file.
    pub fn from_oci_archive<P: AsRef<Path>>(path: P) -> Result<Self, miette::Report> {
        if !path.as_ref().is_file() {
            miette::bail!("OCI archive doesn't exist at {}", path.as_ref().display());
        }

        Ok(Self::OciArchive(path.as_ref().to_owned()))
    }

    /// # Errors
    /// Returns an error if the path does not point to a directory.
    pub fn from_oci_directory<P: AsRef<Path>>(path: P) -> Result<Self, miette::Report> {
        if !path.as_ref().is_dir() {
            miette::bail!("OCI directory doesn't exist at {}", path.as_ref().display());
        }

        Ok(Self::OciDir(path.as_ref().to_owned()))
    }

    #[must_use]
    pub fn to_os_string(&self) -> OsString {
        match self {
            Self::LocalStorage(local_ref) => format!("containers-storage:{local_ref}").into(),
            Self::OciArchive(path) => {
                let mut out = OsString::from("oci-archive:");
                out.push(path.as_os_str());
                out
            }
            Self::OciDir(path) => {
                let mut out = OsString::from("oci:");
                out.push(path.as_os_str());
                out
            }
            Self::Remote(image_ref) => format!("docker://{}", image_ref.whole()).into(),
        }
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

impl<'scope> ImageRef<'scope> {
    #[must_use]
    pub fn remote_ref(&self) -> Option<&Reference> {
        match self {
            Self::Remote(remote) => Some(remote.as_ref()),
            _ => None,
        }
    }

    #[must_use]
    pub fn with_platform(&'scope self, platform: Platform) -> Self {
        if let Self::Remote(remote) = &self {
            Self::Remote(Cow::Owned(platform.tagged_image(remote)))
        } else if let Self::LocalTar(path) = &self
            && let Some(tagged) = platform.tagged_path(path)
        {
            Self::LocalTar(Cow::Owned(tagged))
        } else {
            Self::from(self)
        }
    }

    /// Appends a value to the end of a tag.
    ///
    /// If the ref is a tarball, it will append it to the file
    /// stem. If it's other, it will append to the end of the value.
    #[must_use]
    pub fn append_tag(&self, value: &Tag) -> Self {
        match self {
            Self::Remote(image) => Self::Remote(Cow::Owned(Reference::with_tag(
                image.registry().to_owned(),
                image.repository().to_owned(),
                image
                    .tag()
                    .map_or_else(|| format!("latest_{value}"), |tag| format!("{tag}_{value}")),
            ))),
            Self::LocalTar(path) => {
                if let Some(file_stem) = path.file_stem()
                    && let Some(extension) = path.extension()
                {
                    Self::LocalTar(Cow::Owned(
                        path.with_file_name(format!("{}_{value}", file_stem.display(),))
                            .with_extension(extension),
                    ))
                } else {
                    Self::LocalTar(Cow::Owned(PathBuf::from(format!(
                        "{}_{value}",
                        path.display()
                    ))))
                }
            }
            Self::Other(other) => Self::Other(Cow::Owned(format!("{other}_{value}"))),
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

#[derive(Debug, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tag(String);

impl Tag {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for Tag {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let regex = regex!(r"[\w][\w.-]{0,127}");
        regex
            .is_match(s)
            .then(|| Self(s.into()))
            .ok_or_else(|| miette!("Invalid tag: {s}"))
    }
}

impl std::fmt::Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", &self.0)
    }
}

impl TryFrom<&Reference> for Tag {
    type Error = miette::Error;

    fn try_from(value: &Reference) -> Result<Self, Self::Error> {
        value
            .tag()
            .map(|tag| Self(tag.into()))
            .ok_or_else(|| miette!("Reference {value} has no tag"))
    }
}

impl<'de> Deserialize<'de> for Tag {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Self::from_str(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl Default for Tag {
    fn default() -> Self {
        Self(String::from("latest"))
    }
}

impl From<Tag> for String {
    fn from(value: Tag) -> Self {
        value.0
    }
}

impl From<&Tag> for String {
    fn from(value: &Tag) -> Self {
        value.0.clone()
    }
}
