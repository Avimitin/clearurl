//! clearurl is a re-implementation of the [ClearURLs](https://github.com/ClearURLs/Addon)
//! for the the [Rust](http://rust-lang.org/) programming language. It provides simple API
//! to remove tracking queries to protect your privacy.
//!
//! ## Usage
//!
//! use clearurl::UrlCleaner;
//!
//! #[tokio::main]
//! async fn main() {
//!     let cleaner = UrlCleaner::from_file("/path/to/rules.toml").unwrap();
//!     let result = cleaner.clear("https://b23.tv/C0lw13z").unwrap();
//!     assert_eq!(result, "https://www.bilibili.com/video/BV1GJ411x7h7?p=1")
//! }

#[cfg(feature = "hooks")]
mod hooks;
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
    #[error("Fail to exectute hook {0}: {1}")]
    HookExecutionError(String, String),
}

impl UrlCleaner {
    /// This function read rule data from file. The file must be in toml format.
    ///
    /// # Error
    ///
    /// Return error when IO fail or meeting unexpected format.
    pub fn from_file(path: &str) -> Result<UrlCleaner, reqwest::Error> {
        Ok(UrlCleaner {
            rules: rules::parse_from_file(path),
            // default with HTTP/s proxy and 10 max redirect hop policy
            http_client: reqwest::Client::new(),
        })
    }

    pub fn from_toml(data: &str) -> Result<UrlCleaner, reqwest::Error> {
        Ok(UrlCleaner {
            rules: rules::parse(data),
            http_client: reqwest::Client::new(),
        })
    }

    fn clean(rule: &rules::Rule, url: &Url) -> Result<Url, UrlCleanError> {
        if rule.rules.is_empty() {
            return Err(UrlCleanError::NoMatchRule);
        }

        let Some(query) = url.query() else {
            return Err(UrlCleanError::NoQuery);
        };

        if query.is_empty() {
            return Err(UrlCleanError::NoQuery);
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
                if v.is_empty() {
                    new_url.query_pairs_mut().append_key_only(&k);
                    return;
                }

                new_url.query_pairs_mut().append_pair(&k, &v);
            });

        if let Some(query) = new_url.query() {
            if query == url.query().unwrap() {
                return Err(UrlCleanError::NothingToClear);
            }
        }

        Ok(new_url)
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

        let new_url = match Self::clean(&rule, &url) {
            Ok(new_url) => new_url,
            Err(UrlCleanError::NoQuery) if !rule.post_hooks.is_empty() => url,

            otherwise => return otherwise,
        };

        #[cfg(feature = "hooks")]
        let new_url = rule
            .post_hooks
            .iter()
            .flat_map(|hook_name| Some((hook_name, hooks::POST_HOOKS.get(hook_name)?)))
            .try_fold(new_url.clone(), |prev_url, (hook_name, hook_fn)| {
                hook_fn(&prev_url).map_err(|err| {
                    UrlCleanError::HookExecutionError(hook_name.to_string(), err.to_string())
                })
            })?;

        Ok(new_url)
    }
}

#[tokio::test]
async fn test_filter() {
    let cleaner = UrlCleaner::from_file("./rules.toml").unwrap();

    // * test normal rule
    let url = cleaner.clear(
        "https://www.bilibili.com/video/BV18x411F7MS/?-Arouter=story&buvid=sjdkladjakslddjashikldajsdkl&from_spmid=tm.recommend.0.0&is_story_h5=true&mid=sdajskdajsdkasjdkasjdka%3D%3D&p=1&plat_id=163&share_from=ugc&share_medium=android&share_plat=android&share_session_id=ajdkasd890-0000-1111-2222-33330djas&share_source=COPY&share_tag=s_i&spmid=main.ugc-video-detail-vertical.0.0&timestamp=1111111&unique_k=hkeZH3o&up_id=1343541951&t=42",
    )
    .await
    .unwrap();
    assert_eq!(
        url.as_str(),
        "https://www.bilibili.com/video/BV18x411F7MS/?p=1&t=42"
    );

    // * test redirection
    #[cfg(feature = "hooks")]
    {
        let url = cleaner
            .clear("https://twitter.com/Naniii_0_o/status/1713328832932147227?t=1&s=1")
            .await
            .unwrap();
        assert_eq!(
            url.as_str(),
            "https://fxtwitter.com/Naniii_0_o/status/1713328832932147227"
        );

        let url = cleaner
            .clear("https://x.com/MyHongKongDoll/status/1720308905513787846")
            .await
            .unwrap();
        assert_eq!(
            url.as_str(),
            "https://fixupx.com/MyHongKongDoll/status/1720308905513787846"
        );
    }

    #[cfg(feature = "bilibili_hooks")]
    {
        let url = cleaner.clear("https://b23.tv/uPcjzlS").await.unwrap();
        assert_eq!(
            url.as_str(),
            "https://www.bilibili.com/video/av746592874/?p=1"
        );

        let url = cleaner.clear("https://b23.tv/Cj2HC2K").await.unwrap();
        assert_eq!(
            url.as_str(),
            "https://www.bilibili.com/video/av746592874/?p=1"
        );
    }

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

    // * test key only
    let url = cleaner.clear("https://t.me/example/321?single").await;
    match url {
        Err(UrlCleanError::NothingToClear) => {}
        _ => {
            panic!("URL doesn't return error NothingToClear")
        }
    };
}
