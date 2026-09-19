use std::{
    ops::Deref,
    path::{Path, PathBuf},
};

use miette::{IntoDiagnostic, bail};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NormalizedPath(PathBuf);

impl TryFrom<PathBuf> for NormalizedPath {
    type Error = miette::Error;

    fn try_from(value: PathBuf) -> std::result::Result<Self, Self::Error> {
        value.canonicalize().into_diagnostic().map(Self)
    }
}

impl TryFrom<&Path> for NormalizedPath {
    type Error = miette::Error;

    fn try_from(value: &Path) -> std::result::Result<Self, Self::Error> {
        value.canonicalize().into_diagnostic().map(Self)
    }
}

impl Deref for NormalizedPath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Path> for NormalizedPath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

pub struct AbsolutePath(PathBuf);

impl TryFrom<PathBuf> for AbsolutePath {
    type Error = miette::Error;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        if value.is_absolute() {
            Ok(Self(value))
        } else {
            bail!("Path {} is not absolute", value.display())
        }
    }
}

impl TryFrom<&Path> for AbsolutePath {
    type Error = miette::Error;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        Self::try_from(value.to_path_buf())
    }
}

impl Deref for AbsolutePath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Path> for AbsolutePath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelativePath(PathBuf);

impl TryFrom<PathBuf> for RelativePath {
    type Error = miette::Error;

    fn try_from(value: PathBuf) -> Result<Self, Self::Error> {
        if value.is_absolute() {
            Ok(Self(value))
        } else {
            bail!("Path {} is not absolute", value.display())
        }
    }
}

impl TryFrom<&Path> for RelativePath {
    type Error = miette::Error;

    fn try_from(value: &Path) -> Result<Self, Self::Error> {
        Self::try_from(value.to_path_buf())
    }
}

impl Deref for RelativePath {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Path> for RelativePath {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ContextPath(NormalizedPath, RelativePath);

#[bon::bon]
impl ContextPath {
    /// Create a `ContextPath` from a `NormalizedPath`
    /// given a `RelativePath`. This verifies that the
    /// path  is within the context.
    ///
    /// # Errors
    /// Will error when the path provided is not within
    /// the `NormalizedPath`.
    #[builder]
    pub const fn new(
        /// The context dir where the path should be located.
        ///
        /// # Errors
        /// Will error when the path doesn't exist.
        #[builder(with = |path: impl AsRef<Path>| -> miette::Result<_> {
            NormalizedPath::try_from(path.as_ref())
        })]
        context_dir: NormalizedPath,

        /// The path inside the context dir.
        ///
        /// # Errors
        /// Will error when the path isn't relative.
        #[builder(with = |path: impl AsRef<Path>| -> miette::Result<_> {
            RelativePath::try_from(path.as_ref())
        })]
        path: RelativePath,
    ) -> Self {
        Self(context_dir, path)
    }

    #[must_use]
    pub fn context(&self) -> &Path {
        &self.0
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.1
    }
}
