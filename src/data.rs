use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use serde::{Serialize, Deserialize};
use anyhow::Result;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Providers {
    providers: HashMap<String, Domain>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Domain {
    url_pattern: String,
    complete_provider: bool,
    rules: Vec<String>,
    referral_marketing: Vec<String>,
    raw_rules: Vec<String>,
    exceptions: Vec<String>,
    redirections: Vec<String>,
    force_redirection: bool,
}

impl Providers {
    pub fn new(path: &str) -> Result<Providers> {
        load_data(path)
    }
}

fn load_data(path: &str) -> Result<Providers> {
    let path = Path::new(path);
    let mut file = File::open(&path)?;

    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;

    let providers = serde_json::from_str(&buffer)?;

    Ok(providers)
}

#[test]
fn test_load_data() {
    let data = load_data("./rules/data.min.json").expect("fail to read data.min.json");
    assert_ne!(0, data.providers.len());

    let bili = data.providers.get("m.bilibili.com");
    assert!(bili.is_some());

    let bili = bili.unwrap();
    assert_eq!(vec!["bbid", "ts"], bili.rules);
}
