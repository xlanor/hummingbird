use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InterfaceSettings {
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub full_width_library: bool,
}

impl Default for InterfaceSettings {
    fn default() -> Self {
        Self {
            language: String::new(),
            full_width_library: false,
        }
    }
}
