use std::{ops::Not, path::PathBuf, str::FromStr};

use blue_build_utils::{container::ImageRef, impl_de_fromstr};
use lazy_regex::{regex_if, regex_switch};
use miette::{IntoDiagnostic, bail};
use oci_distribution::Reference;

impl_de_fromstr!(
    DeploymentImageRef,
    ImageTransport,
    RefIndex,
    DockerDaemon,
    DigestAlgorithm,
    StorageSpecifier,
);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeploymentImageRef {
    UnverifiedImage(ImageTransport),
    UnverifiedRegistry(Reference),
    RemoteImage {
        remote: String,
        reference: Reference,
    },
    RemoteRegistry {
        remote: String,
        reference: Reference,
    },
    ImageSigned(ImageTransport),
}

impl<'a> TryFrom<&'a DeploymentImageRef> for ImageRef<'a> {
    type Error = miette::Error;

    fn try_from(value: &'a DeploymentImageRef) -> Result<Self, Self::Error> {
        Ok(match value {
            DeploymentImageRef::UnverifiedImage(
                ImageTransport::Registry(reference)
                | ImageTransport::Docker(reference)
                | ImageTransport::DockerDaemon(DockerDaemon::Reference(reference))
                | ImageTransport::ContainersStorage {
                    storage_specifier: _,
                    reference,
                },
            )
            | DeploymentImageRef::ImageSigned(
                ImageTransport::Registry(reference)
                | ImageTransport::Docker(reference)
                | ImageTransport::DockerDaemon(DockerDaemon::Reference(reference))
                | ImageTransport::ContainersStorage {
                    storage_specifier: _,
                    reference,
                },
            ) => Self::Remote(std::borrow::Cow::Borrowed(reference)),
            DeploymentImageRef::UnverifiedRegistry(reference) => {
                Self::Remote(std::borrow::Cow::Borrowed(reference))
            }
            DeploymentImageRef::UnverifiedImage(ImageTransport::OciArchive {
                path,
                reference: _,
            })
            | DeploymentImageRef::ImageSigned(ImageTransport::OciArchive { path, reference: _ }) => {
                Self::LocalTar(std::borrow::Cow::Borrowed(path))
            }
            _ => bail!("Failed to convert {value} into an image ref"),
        })
    }
}

impl std::fmt::Display for DeploymentImageRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::UnverifiedImage(transport) => format!("ostree-unverified-image:{transport}"),
                Self::UnverifiedRegistry(reference) =>
                    format!("ostree-unverified-registry:{reference}"),
                Self::RemoteImage { remote, reference } =>
                    format!("ostree-remote-image:{remote}:registry:{reference}"),
                Self::RemoteRegistry { remote, reference } =>
                    format!("ostree-remote-registry:{remote}:{reference}"),
                Self::ImageSigned(transport) => format!("ostree-image-signed:{transport}"),
            }
        )
    }
}

impl FromStr for DeploymentImageRef {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        regex_switch!(
            s,
            r"ostree-unverified-image:(?<reference>.*)" => {
                Self::UnverifiedImage(reference.try_into()?)
            }
            r"ostree-unverified-registry:(?<reference>.*)" => {
                Self::UnverifiedRegistry(reference.try_into().into_diagnostic()?)
            }
            r"ostree-remote-image:(?<remote>[^:]+):registry:(?<reference>.*)" => {
                Self::RemoteImage {
                    remote: remote.into(),
                    reference: reference.try_into().into_diagnostic()?,
                }
            }
            r"ostree-remote-registry:(?<remote>[^:]+):(?<reference>.*)" => {
                Self::RemoteRegistry {
                    remote: remote.into(),
                    reference: reference.try_into().into_diagnostic()?,
                }
            }
            r"ostree-image-signed:(?<transport>.*)" => {
                Self::ImageSigned(transport.try_into()?)
            }
        )
        .ok_or_else(|| miette::miette!("Failed to parse '{s}' as an image transport"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageTransport {
    Registry(Reference),
    Docker(Reference),
    DockerArchive {
        path: PathBuf,
        ref_index: Option<RefIndex>,
    },
    DockerDaemon(DockerDaemon),
    Dir(PathBuf),
    Oci {
        path: PathBuf,
        ref_index: Option<RefIndex>,
    },
    OciArchive {
        path: PathBuf,
        reference: Option<Reference>,
    },
    ContainersStorage {
        storage_specifier: Option<StorageSpecifier>,
        reference: Reference,
    },
    Ostree {
        reference: Reference,
        repo_path: Option<PathBuf>,
    },
    Sif(PathBuf),
}

impl std::fmt::Display for ImageTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Registry(reference) => format!("registry:{reference}"),
                Self::Docker(reference) => format!("docker://{reference}"),
                Self::DockerArchive {
                    path,
                    ref_index: None,
                } => format!("docker-archive:{}", path.display()),
                Self::DockerArchive {
                    path,
                    ref_index: Some(ref_index),
                } => format!("docker-archive:{}:{ref_index}", path.display()),
                Self::DockerDaemon(daemon) => format!("docker-daemon:{daemon}"),
                Self::Dir(path) => format!("dir:{}", path.display()),
                Self::Oci {
                    path,
                    ref_index: None,
                } => format!("oci:{}", path.display()),
                Self::Oci {
                    path,
                    ref_index: Some(ref_index),
                } => format!("oci:{}:{ref_index}", path.display()),
                Self::OciArchive {
                    path,
                    reference: None,
                } => format!("oci-archive:{}", path.display()),
                Self::OciArchive {
                    path,
                    reference: Some(reference),
                } => format!("oci-archive:{}:{reference}", path.display()),
                Self::ContainersStorage {
                    storage_specifier: None,
                    reference,
                } => format!("containers-storage:{reference}"),
                Self::ContainersStorage {
                    storage_specifier: Some(storage_specifier),
                    reference,
                } => format!("containers-storage:[{storage_specifier}]{reference}"),
                Self::Ostree {
                    reference,
                    repo_path: None,
                } => format!("ostree:{reference}"),
                Self::Ostree {
                    reference,
                    repo_path: Some(repo_path),
                } => format!("ostree:{reference}@{}", repo_path.display()),
                Self::Sif(path) => format!("sif:{}", path.display()),
            }
        )
    }
}

impl FromStr for ImageTransport {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        regex_switch!(
            s,
            r"registry:(?<reference>.*)" => {
                Self::Registry(reference.try_into().into_diagnostic()?)
            }
            r"docker://(?<reference>.*)" => {
                Self::Docker(reference.try_into().into_diagnostic()?)
            }
            r"docker-archive:(?<path>[^:]+)(?::(?<ref_index>.*))?" => {
                let ref_index = if ref_index.is_empty().not() {
                    Some(ref_index.try_into()?)
                } else {
                    None
                };
                Self::DockerArchive { path: path.into(), ref_index }
            }
            r"docker-daemon:(?<reference>.*)" => {
                Self::DockerDaemon(reference.try_into()?)
            }
            r"dir:(?<path>.*)" => {
                Self::Dir(path.into())
            }
            r"oci:(?<path>[^:]+)(?::(?<ref_index>.*))?" => {
                let ref_index = if ref_index.is_empty().not() {
                    Some(ref_index.try_into()?)
                } else {
                    None
                };
                Self::Oci { path: path.into(), ref_index }
            }
            r"oci-archive:(?<path>[^:]+)(?::(?<reference>.*))?" => {
                let reference = if reference.is_empty().not() {
                    Some(reference.try_into().into_diagnostic()?)
                } else {
                    None
                };
                Self::OciArchive { path: path.into(), reference }
            }
            r"containers-storage:(?:\[(?<storage_specifier>.*)\])?(?<reference>.*)" => {
                let storage_specifier = if storage_specifier.is_empty().not() {
                    Some(storage_specifier.try_into()?)
                } else {
                    None
                };
                Self::ContainersStorage { storage_specifier, reference: reference.parse().into_diagnostic()? }
            }
            r"ostree:(?<reference>[^@]+)(?:@(?<repo_path>.*))?" => {
                let repo_path = if repo_path.is_empty().not() {
                    Some(repo_path.into())
                } else {
                    None
                };
                Self::Ostree { reference: reference.parse().into_diagnostic()?, repo_path }
            }
            r"sif:(?<path>.*)" => {
                Self::Sif(path.into())
            }
        )
        .ok_or_else(|| miette::miette!("Failed to parse '{s}' as an image transport"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefIndex {
    Reference(Reference),
    Index(usize),
}

impl std::fmt::Display for RefIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Reference(reference) => format!("{reference}"),
                Self::Index(index) => format!("{index}"),
            }
        )
    }
}

impl FromStr for RefIndex {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match (Reference::try_from(s), s.parse::<usize>()) {
            (_, Ok(index)) => Self::Index(index),
            (Ok(reference), _) => Self::Reference(reference),
            _ => bail!("Failed to parse '{s}' into a reference or index"),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DockerDaemon {
    Reference(Reference),
    Algo {
        algo: DigestAlgorithm,
        digest: String,
    },
}

impl std::fmt::Display for DockerDaemon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Reference(reference) => format!("{reference}"),
                Self::Algo { algo, digest } => format!(
                    "{}:{digest}",
                    match algo {
                        DigestAlgorithm::Sha256 => "sha256",
                        DigestAlgorithm::Sha384 => "sha384",
                        DigestAlgorithm::Sha512 => "sha512",
                    }
                ),
            }
        )
    }
}

impl FromStr for DockerDaemon {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(
            match (
                s.split_once(':').map(|(algo, digest)| {
                    (
                        DigestAlgorithm::try_from(algo),
                        regex_if!(r"[a-f0-9]+", digest, digest),
                    )
                }),
                Reference::try_from(s),
            ) {
                (Some((Ok(algo), Some(digest))), _) => Self::Algo {
                    algo,
                    digest: digest.into(),
                },
                (_, Ok(reference)) => Self::Reference(reference),
                _ => bail!("Failed to parse '{s}' as a docker daemon reference"),
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DigestAlgorithm {
    Sha256,
    Sha384,
    Sha512,
}

impl FromStr for DigestAlgorithm {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "sha256" => Self::Sha256,
            "sha384" => Self::Sha384,
            "sha512" => Self::Sha512,
            _ => bail!("Failed to parse '{s}' as a digest algorithm"),
        })
    }
}

impl std::fmt::Display for DigestAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Sha256 => "sha256",
                Self::Sha384 => "sha384",
                Self::Sha512 => "sha512",
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageSpecifier {
    driver: Option<String>,
    root: PathBuf,
    run_root: Option<PathBuf>,
    options: Option<String>,
}

impl std::fmt::Display for StorageSpecifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{driver}{root}{run_root}{options}",
            driver = self
                .driver
                .as_ref()
                .map(|d| format!("{d}@"))
                .unwrap_or_default(),
            root = self.root.display(),
            run_root = self
                .run_root
                .as_ref()
                .map(|r| format!("+{}", r.display()))
                .unwrap_or_default(),
            options = self
                .options
                .as_ref()
                .map(|o| format!(":{o}"))
                .unwrap_or_default(),
        )
    }
}

impl FromStr for StorageSpecifier {
    type Err = miette::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        regex_if!(
            r"(?:(?<driver>[\w-]+)@)?(?<root>[\w\/-]+)(?:\+(?<run_root>[\w\/-]+))?(?:\:(?<options>[\w,=-]+))?",
            s,
            {
                Self {
                    driver: driver.is_empty().not().then(|| driver.into()),
                    root: root.into(),
                    run_root: run_root.is_empty().not().then(|| run_root.into()),
                    options: options.is_empty().not().then(|| options.into()),
                }
            }
        )
        .ok_or_else(|| miette::miette!("Failed to parse storage specifier"))
    }
}

#[cfg(test)]
mod test {
    use oci_distribution::Reference;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    use super::*;

    macro_rules! test_parse {
        ($($test:ident {
            typ: $typ:ty,
            value: $val:literal,
            variant: $var:pat$(,)?
        }),* $(,)?) => {
            $(
                #[test]
                fn $test() {
                    let transport = <$typ>::try_from($val).unwrap();
                    assert!(
                        matches!(
                            &transport,
                            $var
                        )
                    );
                    assert_eq!($val, transport.to_string().as_str());
                }
            )*
        };
    }

    test_parse!(
        parse_image_transport_registry {
            typ: ImageTransport,
            value: "registry:ghcr.io/ublue-os/main-kinoite:42",
            variant: ImageTransport::Registry(_),
        },
        parse_image_transport_docker {
            typ: ImageTransport,
            value: "docker://ghcr.io/ublue-os/main-kinoite:42",
            variant: ImageTransport::Docker(_),
        },
        parse_image_transport_docker_archive {
            typ: ImageTransport,
            value: "docker-archive:/test/path",
            variant: ImageTransport::DockerArchive {
                path: _,
                ref_index: None,
            }
        },
        parse_image_transport_docker_archive_index {
            typ: ImageTransport,
            value: "docker-archive:/test/path:42",
            variant: ImageTransport::DockerArchive {
                path: _,
                ref_index: Some(RefIndex::Index(_)),
            }
        },
        parse_image_transport_docker_archive_ref {
            typ: ImageTransport,
            value: "docker-archive:/test/path:ghcr.io/ublue-os/main-kinoite:42",
            variant: ImageTransport::DockerArchive {
                path: _,
                ref_index: Some(RefIndex::Reference(_)),
            }
        },
        parse_image_transport_docker_daemon_ref {
            typ: ImageTransport,
            value: "docker-daemon:ghcr.io/ublue-os/main-kinoite:42",
            variant: ImageTransport::DockerDaemon(DockerDaemon::Reference(_)),
        },
        parse_image_transport_docker_daemon_digest {
            typ: ImageTransport,
            value: "docker-daemon:sha256:e6cbc801b77c4cfe164f08b6b29de7e588f6d98e8ac0c52c0de0a9ae45f717ab",
            variant: ImageTransport::DockerDaemon(DockerDaemon::Algo {
                algo: DigestAlgorithm::Sha256,
                digest: _,
            }),
        },
        parse_image_transport_dir {
            typ: ImageTransport,
            value: "dir:/test/path",
            variant: ImageTransport::Dir(_),
        },
        parse_image_transport_oci {
            typ: ImageTransport,
            value: "oci:/test/path",
            variant: ImageTransport::Oci {
                path: _,
                ref_index: None
            }
        },
        parse_image_transport_oci_ref {
            typ: ImageTransport,
            value: "oci:/test/path:ghcr.io/ublue-os/main-kinoite:42",
            variant: ImageTransport::Oci {
                path: _,
                ref_index: Some(RefIndex::Reference(_)),
            }
        },
        parse_image_transport_oci_ref_index {
            typ: ImageTransport,
            value: "oci:/test/path:42",
            variant: ImageTransport::Oci {
                path: _,
                ref_index: Some(RefIndex::Index(_))
            }
        },
        parse_image_transport_oci_archive {
            typ: ImageTransport,
            value: "oci-archive:/test/path",
            variant: ImageTransport::OciArchive {
                path: _,
                reference: None
            }
        },
        parse_image_transport_oci_archive_ref {
            typ: ImageTransport,
            value: "oci-archive:/test/path:ghcr.io/ublue-os/main-kinoite:42",
            variant: ImageTransport::OciArchive {
                path: _,
                reference: Some(_)
            }
        },
        parse_image_transport_containers_storage {
            typ: ImageTransport,
            value: "containers-storage:ghcr.io/ublue-os/main-kinoite:42",
            variant: ImageTransport::ContainersStorage {
                storage_specifier: None,
                reference: _
            }
        },
        parse_image_transport_containers_storage_specifier {
            typ: ImageTransport,
            value: "containers-storage:[overlayfs@/test/path]ghcr.io/ublue-os/main-kinoite:42",
            variant: ImageTransport::ContainersStorage {
                storage_specifier: Some(StorageSpecifier {
                    driver: Some(_),
                    root: _,
                    run_root: None,
                    options: None
                }),
                reference: _
            }
        },
        parse_image_transport_ostree {
            typ: ImageTransport,
            value: "ostree:ghcr.io/ublue-os/main-kinoite:42",
            variant: ImageTransport::Ostree {
                reference: _,
                repo_path: None
            }
        },
        parse_image_transport_ostree_repo_path {
            typ: ImageTransport,
            value: "ostree:ghcr.io/ublue-os/main-kinoite:42@/test/path",
            variant: ImageTransport::Ostree {
                reference: _,
                repo_path: Some(_)
            }
        },
        parse_image_transport_sif {
            typ: ImageTransport,
            value: "sif:/test/path",
            variant: ImageTransport::Sif(_),
        },
        parse_deployment_image_ref_unverified_image {
            typ: DeploymentImageRef,
            value: "ostree-unverified-image:registry:ghcr.io/ublue-os/main-kinoite:42",
            variant: DeploymentImageRef::UnverifiedImage(ImageTransport::Registry(_)),
        },
        parse_deployment_image_ref_unverified_registry {
            typ: DeploymentImageRef,
            value: "ostree-unverified-registry:ghcr.io/ublue-os/main-kinoite:42",
            variant: DeploymentImageRef::UnverifiedRegistry(_),
        },
        parse_deployment_image_ref_remote_image {
            typ: DeploymentImageRef,
            value: "ostree-remote-image:origin:registry:ghcr.io/ublue-os/main-kinoite:42",
            variant: DeploymentImageRef::RemoteImage {
                remote: _,
                reference: _
            }
        },
        parse_deployment_image_ref_remote_registry {
            typ: DeploymentImageRef,
            value: "ostree-remote-registry:origin:ghcr.io/ublue-os/main-kinoite:42",
            variant: DeploymentImageRef::RemoteRegistry {
                remote: _,
                reference: _
            }
        },
        parse_deployment_image_ref_image_signed {
            typ: DeploymentImageRef,
            value: "ostree-image-signed:registry:ghcr.io/ublue-os/main-kinoite:42",
            variant: DeploymentImageRef::ImageSigned(ImageTransport::Registry(_)),
        }
    );

    #[rstest]
    #[case(
        "ghcr.io/ublue-os/main-kinoite:42",
        Some("ghcr.io/ublue-os/main-kinoite:42".try_into().unwrap()),
        None
    )]
    #[case(
        "sha256:e6cbc801b77c4cfe164f08b6b29de7e588f6d98e8ac0c52c0de0a9ae45f717ab",
        None,
        Some((
            "sha256",
            "e6cbc801b77c4cfe164f08b6b29de7e588f6d98e8ac0c52c0de0a9ae45f717ab",
        ))
    )]
    fn parse_docker_daemon(
        #[case] value: &str,
        #[case] reference: Option<Reference>,
        #[case] algo_digest: Option<(&str, &str)>,
    ) {
        let expected = match (reference, algo_digest) {
            (Some(reference), None) => DockerDaemon::Reference(reference),
            (None, Some((algo, digest))) => DockerDaemon::Algo {
                algo: algo.try_into().unwrap(),
                digest: digest.into(),
            },
            _ => unreachable!(),
        };

        assert_eq!(DockerDaemon::try_from(value).unwrap(), expected);
        assert_eq!(value, &expected.to_string());
    }

    #[rstest]
    #[case("/test/path", None, "/test/path", None, None)]
    #[case("overlayfs@/test/path", Some("overlayfs"), "/test/path", None, None)]
    #[case(
        "/test/path+/test/run/path",
        None,
        "/test/path",
        Some("/test/run/path"),
        None
    )]
    #[case(
        "/test/path:param_1=test,param_2=anotherTest",
        None,
        "/test/path",
        None,
        Some("param_1=test,param_2=anotherTest")
    )]
    #[case(
        "/test/path+/test/run/path:param_1=test,param_2=anotherTest",
        None,
        "/test/path",
        Some("/test/run/path"),
        Some("param_1=test,param_2=anotherTest")
    )]
    #[case(
        "overlayfs@/test/path+/test/run/path",
        Some("overlayfs"),
        "/test/path",
        Some("/test/run/path"),
        None
    )]
    #[case(
        "overlayfs@/test/path:param_1=test,param_2=anotherTest",
        Some("overlayfs"),
        "/test/path",
        None,
        Some("param_1=test,param_2=anotherTest")
    )]
    #[case(
        "overlayfs@/test/path+/test/run/path:param_1=test,param_2=anotherTest",
        Some("overlayfs"),
        "/test/path",
        Some("/test/run/path"),
        Some("param_1=test,param_2=anotherTest")
    )]
    fn parse_storage_specifier(
        #[case] value: &str,
        #[case] driver: Option<&str>,
        #[case] root: &str,
        #[case] run_root: Option<&str>,
        #[case] options: Option<&str>,
    ) {
        let expected = StorageSpecifier {
            driver: driver.map(Into::into),
            root: root.into(),
            run_root: run_root.map(Into::into),
            options: options.map(Into::into),
        };

        assert_eq!(StorageSpecifier::try_from(value).unwrap(), expected);
        assert_eq!(value, expected.to_string().as_str());
    }
}
