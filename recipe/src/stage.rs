use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use crate::ModuleExt;

#[derive(Serialize, Deserialize, Debug, Clone, TypedBuilder)]
pub struct Stage<'a> {
    #[builder(setter(into))]
    pub name: Cow<'a, str>,

    #[builder(setter(into))]
    pub image: Cow<'a, str>,

    #[serde(flatten)]
    pub modules_ext: ModuleExt<'a>,
}
