use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MountCacheSharing {
    /// The cache is shared between all builds.
    #[serde(rename = "shared")]
    Shared,

    /// The cache is private to the current build.
    #[serde(rename = "private")]
    Private,

    /// The cache is shared between builds, but only one build can use it at a time.
    #[serde(rename = "locked")]
    Locked,
}
impl std::fmt::Display for MountCacheSharing {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Shared => write!(f, "shared"),
            Self::Private => write!(f, "private"),
            Self::Locked => write!(f, "locked"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Mount {
    /// A bind mount, which mounts a file or directory from the host.
    #[serde(rename = "bind")]
    Bind {
        /// The source path on the host.
        source: String,
        /// The destination path in the container.
        destination: String,
        /// Whether the mount is read-only.
        #[serde(default, rename = "readonly")]
        readonly: bool,
    },

    /// A tmpfs mount, which mounts a temporary file system in memory.
    #[serde(rename = "tmpfs")]
    Tmpfs {
        /// The destination path.
        destination: String,
        /// The size of the tmpfs. Can be specified in bytes or with a suffix (e.g. "100m" for 100 megabytes).
        #[serde(skip_serializing_if = "Option::is_none")]
        size: Option<String>,
    },

    /// A cache mount, which mounts a cache directory that can be shared between builds.
    #[serde(rename = "cache")]
    Cache {
        /// The destination path.
        destination: String,
        /// The cache ID, which is used to identify the cache.
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// The cache sharing mode.
        #[serde(skip_serializing_if = "Option::is_none")]
        sharing: Option<MountCacheSharing>,
    },
}

impl std::fmt::Display for Mount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bind {
                source,
                destination,
                readonly,
            } => {
                write!(f, "type=bind,source={source},dst={destination}")?;
                if *readonly {
                    write!(f, ",readonly")?;
                }
            }
            Self::Tmpfs { destination, size } => {
                write!(f, "type=tmpfs,dst={destination}")?;
                if let Some(size) = size {
                    write!(f, ",size={size}")?;
                }
            }
            Self::Cache {
                destination,
                id,
                sharing,
            } => {
                write!(f, "type=cache")?;
                if let Some(sharing) = sharing {
                    write!(f, ",sharing={sharing}")?;
                }
                write!(f, ",dst={destination}")?;
                if let Some(id) = id {
                    write!(f, ",id={id}")?;
                }
            }
        }
        Ok(())
    }
}

impl Mount {
    #[must_use]
    pub const fn oci_suffix(&self) -> &'static str {
        match self {
            Self::Bind { .. } => ",z",
            _ => "",
        }
    }
}
