use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

// Rule for single domain
#[derive(Serialize, Deserialize, Debug)]
pub struct DomainConfig {
    pub match_sub: bool,
    pub should_redirect: bool,
    pub import: String,
    pub rules: Vec<String>,
}

// A parsed domain rule set
pub struct Domains {
    data: HashMap<String, DomainConfig>,
}

impl Domains {
    /// Load rules for domain from given file. The file must be in toml format.
    ///
    /// # Errors
    ///
    /// This function return error when IO fail or parse progress fail.
    pub fn load_from_file(path: &str) -> Result<Domains> {
        let mut raw = std::fs::read(path).context(format!("Fail to read from file {}", path))?;

        let data: HashMap<String, DomainConfig> = toml::from_str(
            std::str::from_utf8(&raw)
                .context(format!("fail to convert file {} to string literal", path))?,
        )?;

        Ok(Domains { data })
    }

    /// Get return the rule for given domain
    ///
    /// # Example
    /// ```
    /// use clearurl::Domains;
    ///
    /// let domain = "b23.tv";
    /// let domain_ruleset = Domains::load_from_file("./rules.toml").unwrap();
    /// let domain_rule = domain_ruleset.get(domain).unwrap();
    ///
    /// assert!(domain_rule.should_redirect);
    /// ```
    pub fn get(&self, key: &str) -> Option<&DomainConfig> {
        self.data.get(key)
    }

    pub fn amount(&self) -> usize {
        self.data.len()
    }
}

#[test]
fn test_load_data() {
    let data = Domains::load_from_file("./rules.toml").expect("fail to read rules file");
    assert_ne!(0, data.amount());

    let bili = data.get("_test");
    assert!(bili.is_some());

    let bili = bili.unwrap();
    assert_eq!(vec!["_field1", "_field2"], bili.rules);
}
