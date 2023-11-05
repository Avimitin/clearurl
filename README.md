# clearurl

This is a Rust implementation of the ClearURL lib.

## Features

* Declarative configuration
* Full regexp support
* 302 redirect support
* Batch apply configuration for sub domains
* Post hook for rewriting URL

## Usage

- Library

```toml
# Cargo.toml

clearurl = { version = "0.6", features = ["hooks"] }
```

```rust
use clearurl::URLCleaner;

#[tokio::main]
async fn main() {
  std::fs::write("rules.toml", r#"
["b23.tv"]
redirect = true

["bilibili.com"]
sub = ["www", "live", "m"]
ban = [
  "-Arouter",
  "bbid",
  "buvid",
  "callback",
  "from.*",
  "is_story_h5",
  "mid",
  "msource",
  "plat_id",
  "refer_from",
  "seid",
  "share.*",
  "spm_id.*",
  "timestamp",
  "ts",
  "unique_k",
  "up_id",
  "vd_source",
]
post_hooks = [ "bv_to_av" ]
  "#).unwrap();
  let cleaner = URLCleaner::from_file("./rules.toml").unwrap();

  let url = "https://b23.tv/C0lw13z";
  cleaner.clear(url).await.unwrap();

  let url = cleaner.clear("https://b23.tv/Cj2HC2K").await.unwrap();
  assert_eq!(
      url.as_str(),
      "https://www.bilibili.com/video/av746592874/?p=1"
  );
}
```
