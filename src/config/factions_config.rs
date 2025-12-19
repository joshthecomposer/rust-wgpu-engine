use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Deserialize, Debug, Serialize)]
pub struct FactionsConfig {
    pub factions: HashSet<String>,
}

impl Default for FactionsConfig {
    fn default() -> Self {
        Self {
            factions: HashSet::new(),
        }
    }
}

impl Config for FactionsConfig {}
