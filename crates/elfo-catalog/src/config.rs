use std::path::Path;

use elfo_core::forces::ForceModel;
use serde::Deserialize;

/// One force-model/combo entry driving a family sweep.
#[derive(Debug, Clone, Deserialize)]
pub struct ComboCfg {
    pub id: String,
    pub name: String,
    pub force_model: ForceModel,
}

/// Top-level catalog-generation config, loaded from TOML.
#[derive(Debug, Clone, Deserialize)]
pub struct CatalogConfig {
    pub combos: Vec<ComboCfg>,
    pub resonances: Vec<u32>,
    pub members_per_direction: usize,
    pub ds0: f64,
}

impl CatalogConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }
}
