use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use crate::Stage;

#[derive(Default, Serialize, Clone, Deserialize, Debug, TypedBuilder)]
pub struct StagesExt<'a> {
    pub stages: Cow<'a, [Stage<'a>]>,
}
