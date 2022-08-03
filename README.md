# clearurl

This is a Rust implementation of the ClearURL.

## Usage

```toml
# Cargo.toml

clearurl = "0.5"
```

```rust
use clearurl::URLCleaner;

#[tokio::main]
async fn main() {
  let cleaner = URLCleaner::from_file("./rules.toml").unwrap();

  let url = "https://b23.tv/C0lw13z";
  cleaner.clear(url).await.unwrap();

  assert_eq!(
      url.as_str(),
      // normal queries will be kept
      "https://www.bilibili.com/video/BV1GJ411x7h7?p=1"
  );

  println!("Clean URL: {}", url);
}
```

```bash
wget https://raw.githubusercontent.com/Avimitin/clearurl/master/rules.toml

cargo run --release
```
