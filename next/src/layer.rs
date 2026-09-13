mod build_step;
mod copy;
mod from;
mod mount;
mod run;
mod unshare;

use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    hash::Hash,
    process::Command,
    str::FromStr,
    sync::Arc,
};

use blue_build_utils::cmd_out;
use comlexr::{cmd, cmd_mut};
use lazy_regex::regex;
use miette::{Diagnostic, Result};
use serde::Deserialize;
use thiserror::Error;

use crate::layer::build_step::{Container, StepBuilder};

pub use copy::*;
pub use from::*;
pub use mount::*;
pub use run::*;
pub use unshare::*;

/// Build a `Layer`.
trait BuildLayer: Sized {
    /// This function runs the `buildah` command
    /// for the `Layer` type.
    fn run(&self, container: &Container) -> Result<()>;

    /// Build the image, producing a `BuildStep`.
    fn build(&self) -> Result<Box<dyn StepBuilder>>;
}

/// Finalize a build and return the `LayerId`
/// of the image.
#[expect(private_bounds)]
pub trait FinalizeLayer: BuildLayer {
    /// Finalize the layer by running the build graph
    /// and returning the `LayerId` that can be used
    /// to create another build graph.
    ///
    /// # Errors
    /// Will error if the build fails.
    fn finalize(self) -> Result<LayerId> {
        let step = self.build()?;
        step.finalize()
    }
}

impl FinalizeLayer for Arc<FromLayer> {}
impl FinalizeLayer for Arc<CopyLayer> {}
impl FinalizeLayer for Arc<RunLayer> {}

/// An enumeration of all the possible `Layer`s
/// that can be used in the build graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Layer {
    /// The `FromLayer`.
    From(Arc<FromLayer>),

    /// The `RunLayer`.
    Run(Arc<RunLayer>),

    /// The `CopyLayer`.
    Copy(Arc<CopyLayer>),

    /// The `UnshareLayer`
    Unshare(Arc<UnshareLayer>),
}

impl FinalizeLayer for Layer {}

impl Layer {
    fn get_root(&self) -> Arc<FromLayer> {
        match self {
            Self::From(from) => from.clone(),
            Self::Run(run) => run.parent().get_root(),
            Self::Copy(copy) => copy.parent().get_root(),
            Self::Unshare(unshare) => unshare.parent().get_root(),
        }
    }
}

macro_rules! layers {
    ($($typ:ty => $var:ident),* $(,)?) => {
        impl BuildLayer for Layer {
            fn build(&self) -> Result<Box<dyn StepBuilder>> {
                match self {
                    $(Self::$var(layer) => layer.build(),)*
                }
            }

            fn run(&self, container: &Container) -> Result<()> {
                match self {
                    $(Self::$var(layer) => layer.run(container),)*
                }
            }
        }

        $(
        impl From<Arc<$typ>> for Layer {
            fn from(value: Arc<$typ>) -> Self {
                Self::$var(value)
            }
        }

        impl From<$typ> for Layer {
            fn from(value: $typ) -> Self {
                Self::$var(Arc::new(value))
            }
        }
        )*
    };
}

layers!(
    FromLayer => From,
    RunLayer => Run,
    CopyLayer => Copy,
    UnshareLayer => Unshare,
);

/// The ID of a commited `Layer`.
///
/// Can be used for mounts, other images, etc.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayerId(Arc<String>);

#[bon::bon]
impl LayerId {
    #[builder]
    fn new(#[builder(into)] container: Container) -> Result<Self> {
        let layer = cmd_out!(
            parse = Self;
            err_msg = format!("Failed to commit layer for {container}");
            "buildah",
            "commit",
            "--rm",
            &container,
        )?;
        drop(container);
        Ok(layer)
    }

    // #[builder(finish_fn = "build")]
    // fn from_forked(container: &ForkedContainer) -> Result<Self> {
    //     let layer = cmd
    // }
}

/// The error for a badly parsed `LayerId`.
#[derive(Error, Diagnostic, Debug)]
#[error("Invalid layer ID {}", .0)]
pub struct LayerIdParseError(String);

impl FromStr for LayerId {
    type Err = LayerIdParseError;

    fn from_str(value: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        let value = value.trim();

        if regex!("^[a-f0-9]{64}$").is_match(value) {
            Ok(Self(Arc::new(value.trim().to_string())))
        } else {
            Err(LayerIdParseError(value.to_string()))
        }
    }
}

impl std::fmt::Display for LayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'de> Deserialize<'de> for LayerId {
    fn deserialize<D>(deserializer: D) -> std::prelude::v1::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .trim()
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

impl AsRef<OsStr> for LayerId {
    fn as_ref(&self) -> &OsStr {
        (*self.0).as_ref()
    }
}

/// A clonable type to store in `Layer` objets.
///
/// Can be easily converted back and forth
/// from a `Command` object. By using the `Command`
/// type, this allows us to create a `ToString`
/// implementation out of the `Debug` print.
/// `Command`'s `Debug` print is shell-ready and
/// does all the necessary escapes for quotes.
///
/// If you need to run a command inside a shell,
/// you can build something easily using `comlexr`:
///
/// ```rust
/// # use blue_build_next_builder::layer::LayerCommand;
/// let shell = LayerCommand::from(
///     comlexr::cmd!(
///         "/bin/sh",
///         "-c",
///         LayerCommand::from(
///             comlexr::cmd!("echo", "I'm running a command")
///         ).to_string(),
///     )
/// );
/// ```
///
/// The only properties of a `Command` that are retained
/// are `program`, `args`, and `envs`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LayerCommand {
    program: OsString,
    args: Vec<OsString>,
    envs: BTreeMap<OsString, OsString>,
}

impl From<Command> for LayerCommand {
    fn from(value: Command) -> Self {
        Self {
            program: value.get_program().to_owned(),
            args: value.get_args().map(ToOwned::to_owned).collect(),
            envs: value
                .get_envs()
                .filter_map(|(key, value)| value.map(|value| (key.to_owned(), value.to_owned())))
                .collect(),
        }
    }
}

impl From<&LayerCommand> for Command {
    fn from(value: &LayerCommand) -> Self {
        let mut c = cmd!(
            &value.program,
            for &value.args,
        );

        for (key, value) in &value.envs {
            cmd_mut!(
                env {
                    key: value,
                };
                &mut c,
            );
        }
        c
    }
}

impl std::fmt::Display for LayerCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", Command::from(self))
    }
}
