use typed_builder::TypedBuilder;

#[derive(Debug, Clone, TypedBuilder, PartialEq, Eq, Hash)]
pub struct AkmodsInfo {
    pub images: (String, Option<String>),
    pub stage_name: String,
}
