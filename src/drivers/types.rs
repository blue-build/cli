use clap::ValueEnum;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum InspectDriverType {
    Skopeo,
    Podman,
    Docker,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BuildDriverType {
    Buildah,
    Podman,
    Docker,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum RunDriverType {
    Podman,
    Docker,
}

impl From<RunDriverType> for String {
    fn from(value: RunDriverType) -> Self {
        match value {
            RunDriverType::Podman => "podman".to_string(),
            RunDriverType::Docker => "docker".to_string(),
        }
    }
}
