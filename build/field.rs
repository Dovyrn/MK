use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Field {
    pub name: String,
    pub kind: String,
    #[serde(default)]
    pub value: String,
}
