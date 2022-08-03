// The MIT License (MIT)
//
// Copyright (c) 2019-2022 Avimitin
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

/*!
clearurl is a re-implementation of the [ClearURLs](https://github.com/ClearURLs/Addon)
for the the [Rust](http://rust-lang.org/) programming language. It provides simple API
to remove tracking queries to protect your privacy.

## Usage

use clearurl::UrlCleaner;

#[tokio::main]
async fn main() {
    let cleaner = UrlCleaner::from_file("/path/to/rules.toml").unwrap();
    let result = cleaner.clear("https://b23.tv/C0lw13z").unwrap();
    assert_eq!(result, "https://www.bilibili.com/video/BV1GJ411x7h7?p=1")
}
*/

mod filter;
mod rules;

use url::Url;

/// UrlCleaner is a convenient struct which wrap the ruleset data and
/// corresbonding function together.
pub struct UrlCleaner {
    /// ruleset contains rules for domain
    rt: rules::RuntimeRules,
}

impl UrlCleaner {
    /// This function read rule data from file. The file must be in toml format.
    ///
    /// # Error
    ///
    /// Return error when IO fail or meeting unexpected format.
    pub fn from_file(path: &str) -> anyhow::Result<UrlCleaner> {
        Ok(UrlCleaner {
            rt: rules::parse(std::path::Path::new(path)),
        })
    }

    /// The clear function accepct a url string and try to parse it into a new
    /// Url struct without tracking queries.
    pub async fn clear(&self, url: &str) -> Option<Url> {
        match filter::clear(url, &self.rt).await {
            Ok(url) => Some(url),
            Err(e) => {
                tracing::trace!("Error occur when filtering url {}: {}", url, e);
                None
            }
        }
    }
}

// vim: tw=80 fo+=t
