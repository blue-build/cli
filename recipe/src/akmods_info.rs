use bon::Builder;

#[derive(Debug, Clone, Builder, PartialEq, Eq, Hash)]
pub struct AkmodsInfo {
    #[builder(into)]
    pub images: (String, String, Option<String>),

    #[builder(into)]
    pub stage_name: String,
}
