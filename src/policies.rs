use serde::Deserialize;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct PreservePolicy {
    pub retention: String,
    pub min: PreservePolicyMin,
}

#[derive(Debug, Deserialize)]
pub enum RetentionPolicy {
    No,
    Policy(String),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum PreservePolicyMin {
    Variant(PreservePolicyMinVariants),
    Timespan(String),
    Count(usize),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub enum PreservePolicyMinVariants {
    #[serde(alias = "all")]
    All,
    #[serde(alias = "latest")]
    Latest,
}
