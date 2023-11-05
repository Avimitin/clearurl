use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::Path;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Debug)]
struct ConfigData {
    #[serde(default)]
    sub: Option<Vec<String>>,
    #[serde(default)]
    redirect: bool,
    #[serde(default)]
    ban: Vec<String>,
    #[serde(default)]
    post_hooks: Option<Vec<String>>,
}

/// Represent rule for a single domain.
#[derive(Clone, Debug)]
pub struct Rule {
    pub redirect: bool,
    pub rules: Vec<regex::Regex>,
    pub post_hooks: Vec<String>,
}

/// Rules is a KV map with K as full-formed URL, V as clean rules.
pub type Rules = HashMap<String, Arc<Rule>>;

pub fn parse_from_file<P: AsRef<Path> + Debug>(path: P) -> Rules {
    let content = std::fs::read_to_string(path.as_ref())
        .unwrap_or_else(|error| panic!("fail to read from {path:?}: {error}"));
    parse(&content)
}

/// Parse rules configuration file from given `location`.
///
/// # Error
///
/// Panic if
///   * fail to read the file content
///   * fail to parse content into expected struct
///   * regexp is invalid
pub fn parse(content: &str) -> Rules {
    let config: HashMap<String, ConfigData> = toml::from_str(&content)
        .unwrap_or_else(|error| panic!("fail to parse data into rules: {error}"));

    let mut rules = HashMap::new();
    config.into_iter().for_each(|(base, data)| {
        let rule = Arc::new(Rule {
            redirect: data.redirect,
            rules: data
                .ban
                .into_iter()
                .map(|re| {
                    // Use `unwrap_or_else()` instead of `expect` to avoid overhead
                    regex::Regex::new(&re).unwrap_or_else(|error| {
                        panic!("Invalid regexp: '{re}' for URL: {base}\n\nError: {error}")
                    })
                })
                .collect(),
            post_hooks: data.post_hooks.unwrap_or_default(),
        });
        if let Some(sub) = data.sub {
            sub.into_iter().for_each(|sub_domain| {
                rules.insert(format!("{sub_domain}.{base}"), Arc::clone(&rule));
            })
        } else {
            rules.insert(base, rule);
        }
    });

    rules
}
