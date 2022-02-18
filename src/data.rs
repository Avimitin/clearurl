use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

/// RulesStorage store rules for domain.
/// It embed a hashmap nd expose limited hashmap function to guarantee
/// runtime robustness.
pub struct RulesStorage(HashMap<String, DomainConfig>);

impl RulesStorage {
    /// Load rules for domain from given file. The file must be in toml format.
    ///
    /// # Errors
    ///
    /// This function return error when IO fail or parse progress fail.
    pub fn load_from_file(path: &str) -> Result<RulesStorage> {
        let mut raw = std::fs::read(path).context(format!("Fail to read from file {}", path))?;

        let data: HashMap<String, DomainConfig> = toml::from_str(
            std::str::from_utf8(&raw)
                .context(format!("fail to convert file {} to string literal", path))?,
        )?;

        Ok(RulesStorage(data))
    }

    /// Get return the rule for given domain
    pub fn get(&self, key: &str) -> Option<&DomainConfig> {
        self.0.get(key)
    }

    pub fn amount(&self) -> usize {
        self.0.len()
    }
}

// Rule for single domain
#[derive(Serialize, Deserialize, Debug)]
pub struct DomainConfig {
    pub match_sub: bool,
    pub should_redirect: bool,
    pub import: String,
    pub rules: Vec<String>,
}

impl DomainConfig {
    /// Return true if this domain needs to import rules from other domain
    pub fn has_import(&self) -> bool {
        self.import.is_empty()
    }

    /// Return true if there is no rule for this domain
    pub fn has_rules(&self) -> bool {
        self.rules.is_empty()
    }
}

#[test]
fn test_load_data() {
    let data = RulesStorage::load_from_file("./rules.toml").expect("fail to read rules file");
    assert_ne!(0, data.amount());

    let bili = data.get("_test");
    assert!(bili.is_some());

    let bili = bili.unwrap();
    assert_eq!(vec!["_field1", "_field2"], bili.rules);
}
