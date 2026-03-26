use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::config::Config;

/// Configuration mapping weapon types to ability pools.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WeaponAnimMapHelper {
    pub weapon_types: HashMap<String, WeaponActionsHelper>,
}

impl Config for WeaponAnimMapHelper {}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WeaponActionsHelper {
    pub basic_chain: Vec<String>,
    pub dash: String,
    pub block: String,
}
