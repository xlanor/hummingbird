use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InterfaceSettings {
    #[serde(default)]
    pub language: String,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            language: String::new(),
        }
    }
}
