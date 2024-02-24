use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    env, fs,
    path::{Path, PathBuf},
    process::{self, Command},
};

use anyhow::Result;
use chrono::Local;
use format_serde_error::SerdeError;
use indexmap::IndexMap;
use log::{debug, error, info, trace, warn};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use serde_yaml::Value;
use typed_builder::TypedBuilder;

use crate::{
    akmods_info::AkmodsInfo,
    constants::*,
    ops::{self, check_command_exists},
};
