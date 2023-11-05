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

mod rules;

use std::sync::Arc;

use url::Url;

/// UrlCleaner is a convenient struct which wrap the ruleset data and
/// corresbonding function together.
pub struct UrlCleaner {
    /// ruleset contains rules for domain
    rules: rules::Rules,
    http_client: reqwest::Client,
}

#[derive(Debug, thiserror::Error)]
pub enum UrlCleanError {
    #[error("fail to parse input URL")]
    UrlParseError(#[from] url::ParseError),
    #[error("URL have no domain")]
    NoDomain,
    #[error("URL doesn't have any query")]
    NoQuery,
    #[error("fail to do a redirect when cleaning URL")]
    RedirectFail(#[from] reqwest::Error),
    #[error("no rule match for this URL")]
    NoMatchRule,
    #[error("this URL is already cleared")]
    NothingToClear,
}

impl UrlCleaner {
    /// This function read rule data from file. The file must be in toml format.
    ///
    /// # Error
    ///
    /// Return error when IO fail or meeting unexpected format.
    pub fn from_file(path: &str) -> Result<UrlCleaner, reqwest::Error> {
        let mut http_client = reqwest::ClientBuilder::new();

        if let Ok(var) = std::env::var("HTTP_PROXY") {
            http_client = http_client.proxy(reqwest::Proxy::http(var)?);
        };

        if let Ok(var) = std::env::var("HTTPS_PROXY") {
            http_client = http_client.proxy(reqwest::Proxy::https(var)?);
        };

        if let Ok(var) = std::env::var("ALL_PROXY") {
            http_client = http_client.proxy(reqwest::Proxy::all(var)?);
        };

        Ok(UrlCleaner {
            rules: rules::parse(std::path::Path::new(path)),
            http_client: http_client.build()?,
        })
    }

    /// Clear the query of the given URL by pre-define rules.
    ///
    /// # Error
    ///
    /// Return error if:
    ///     * url is invalid
    ///     * no rule found for the given URL and default rule is also not found
    ///     * no query behind the url
    ///     * rule for the given url is empty
    pub async fn clear(&self, url: &str) -> Result<Url, UrlCleanError> {
        let mut url = Url::parse(url)?;

        let get_rule = {
            #[inline]
            |domain: &str| -> Arc<rules::Rule> {
                self.rules
                    .get(domain)
                    .cloned()
                    .unwrap_or(self.rules.get("default").cloned().unwrap())
            }
        };

        let mut domain = url.domain().ok_or_else(|| UrlCleanError::NoDomain)?;
        let mut rule = get_rule(domain);

        if rule.redirect {
            url = self.http_client.head(url).send().await?.url().clone();
            domain = url.domain().unwrap();
            rule = get_rule(domain);
        }

        if url.query().is_none() {
            return Err(UrlCleanError::NoQuery);
        }

        if url.query().unwrap().is_empty() {
            return Err(UrlCleanError::NoQuery);
        }

        if rule.rules.is_empty() {
            return Err(UrlCleanError::NoMatchRule);
        }

        let mut new_url = url.clone();
        new_url.set_query(None);
        url.query_pairs()
            .filter(|(k, _)| {
                let mut is_clean = true;
                for re in &rule.rules {
                    if re.is_match(k) {
                        is_clean = false;
                        break;
                    }
                }
                is_clean
            })
            .for_each(|(k, v)| {
                new_url.query_pairs_mut().append_pair(&k, &v);
            });

        if let Some(query) = new_url.query() {
            if query == url.query().unwrap() {
                return Err(UrlCleanError::NothingToClear);
            }
        }

        Ok(new_url)
    }
}

#[tokio::test]
async fn test_filter() {
    let cleaner = UrlCleaner::from_file("./rules.toml").unwrap();

    // * test normal rule
    let url = cleaner.clear(
        "https://www.bilibili.com/video/BV18c411f75F/?-Arouter=story&buvid=XUA26FCA524D1B63D221F4D6DE86A9EDCC84A&from_spmid=tm.recommend.0.0&is_story_h5=true&mid=7guN1WLkkGNxM7XOufwKvQ%3D%3D&p=1&plat_id=163&share_from=ugc&share_medium=android&share_plat=android&share_session_id=0237255d-d385-49df-861f-b303e20bef5b&share_source=COPY&share_tag=s_i&spmid=main.ugc-video-detail-vertical.0.0&timestamp=1699071016&unique_k=hkeZH3o&up_id=1343541951&t=1230",
    )
    .await
    .unwrap();
    assert_eq!(
        url.as_str(),
        "https://www.bilibili.com/video/BV18c411f75F/?p=1&t=1230"
    );

    // * test redirection
    let url = cleaner.clear("https://b23.tv/acFF8P0").await.unwrap();
    assert_eq!(
        url.as_str(),
        // normal queries will be kept
        "https://www.bilibili.com/video/BV1nY411r7o1/?p=1"
    );

    // * test regex
    let url = cleaner.clear(
        "https://www.amazon.com/b/?node=226184&ref_=Oct_d_odnav_d_1077068_1&pd_rd_w=ZjwFQ&pf_rd_p=0f6f8a08-29ea-497e-8cb4-0ccf91422740&pf_rd_r=YMQ5XPAZHYHV77HCENY7&pd_rd_r=27c502f2-951f-4a8c-9478-381febc5e5bc&pd_rd_wg=NxaQ1",
    )
    .await
    .unwrap();
    assert_eq!(
        url.as_str(),
        // normal queries will be kept
        "https://www.amazon.com/b/?node=226184"
    );

    // * test import
    let url = cleaner.clear(
        "https://post.m.smzdm.com/p/aoxzv08r/?zdm_ss=iOS__hczZ7LgGInW%2BUXtAcwyZGSVdJqcPFvT98aEipRx9K%2BPOH7mQ0YGD3w%3D%3D&from=other",
    )
    .await
    .unwrap();
    assert_eq!(
        url.as_str(),
        // normal queries will be kept
        "https://post.m.smzdm.com/p/aoxzv08r/",
    );

    // * test default
    let url = cleaner
        .clear("https://example.com?utm_source=ios")
        .await
        .unwrap();
    assert_eq!(
        url.as_str(),
        // normal queries will be kept
        "https://example.com/",
    );

    // * test default
    let url = cleaner.clear("https://www.youtube.com/watch?v=FqT_Ofd54fo&list=PLXSyc11qLa1YfSbP700GXf5VSvpVm2zMO&index=42")
        .await;
    match url {
        Err(UrlCleanError::NothingToClear) => {}
        _ => {
            panic!("URL doesn't return error NothingToClear")
        }
    }
}
