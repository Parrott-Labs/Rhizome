use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_borders")]
    pub borders: BorderStyle,
    #[serde(default = "default_density")]
    pub density: Density,
    #[serde(default = "default_signature")]
    pub signature: SignatureWidget,
    #[serde(default = "default_true")]
    pub animations: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle { None, Thin, Mixed, Heavy }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Density { Dense, Balanced, Spacious }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SignatureWidget { Graph, Heat, Ascii, Clean }

fn default_borders()   -> BorderStyle    { BorderStyle::Thin }
fn default_density()   -> Density        { Density::Spacious }
fn default_signature() -> SignatureWidget { SignatureWidget::Graph }
fn default_true()      -> bool            { true }

impl Default for Config {
    fn default() -> Self {
        Self {
            borders:    default_borders(),
            density:    default_density(),
            signature:  default_signature(),
            animations: true,
        }
    }
}

impl Config {
    pub fn load_or_default(path: &Path) -> Self {
        if path.exists()
            && let Ok(text) = std::fs::read_to_string(path)
                && let Ok(cfg) = toml::from_str(&text) {
                    return cfg;
                }
        Self::default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self).unwrap_or_default();
        crate::storage::atomic_write(path, text.as_bytes())
    }
}
