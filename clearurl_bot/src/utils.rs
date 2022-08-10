use anyhow::Result;
use std::sync::Arc;

lazy_static::lazy_static!(
    static ref REGEX_RULE: regex::Regex =
        regex::Regex::new(
            r#"(http[s]?://(?:[a-zA-Z]|[0-9]|[$-_@.&+]|[!*\(\),]|(?:%[0-9a-fA-F][0-9a-fA-F]))+)"#
        ).unwrap();
);

pub fn replace(text: &str, cleaner: &Arc<clearurl::UrlCleaner>) -> Result<String> {
    let result = REGEX_RULE.replace_all(text, |caps: &regex::Captures| {
        let original = &caps[1];
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(cleaner.clear(original))
                .map(|url| url.to_string())
                .unwrap_or_else(|| original.to_string())
        })
    });

    Ok(result.to_string())
}

pub fn capture_url(url: &str) -> Vec<&str> {
    let captures = REGEX_RULE.captures_iter(url);
    let mut v = Vec::new();

    for cap in captures {
        if let Some(c) = cap.get(1) {
            v.push(c.as_str())
        }
    }

    v
}

/// Result that contains the cleared URL, amount of the URL we met and amount of the
/// URL that we modified.
#[derive(Debug)]
pub struct CleanResult {
    pub data: Vec<url::Url>,
    pub met: u32,
    pub cleaned: u32,
}

pub async fn clean(text: &str, cleaner: &Arc<clearurl::UrlCleaner>) -> Result<CleanResult> {
    let urls = capture_url(text);
    if urls.is_empty() {
        anyhow::bail!("no url found in text")
    }

    // amount of the extraced url
    let met = urls.len() as u32;

    let mut data = Vec::new();

    for url in urls {
        if let Some(mut result) = cleaner.clear(url).await {
            if result.as_str() == url {
                continue;
            }

            // change twitter to vxtwitter for better preview
            if let Some("twitter.com") = result.domain() {
                result
                    .set_host(Some("vxtwitter.com"))
                    .unwrap_or_else(|_| panic!("fail to set host to vxtwitter, original: {url}"));
            }

            data.push(result);
        }
    }

    // amount of the modified url
    let cleaned = data.len() as u32;

    Ok(CleanResult { data, met, cleaned })
}

#[tokio::test(flavor = "multi_thread")]
async fn test_replace() {
    let original = "Lorem ipsum dolor sit amet, officia excepteur ex fugiat reprehenderit enim
        labore culpa sint ad nisi Lorem pariatur mollit ex esse exercitation amet. Nisi anim
        https://twitter.com/CiloRanko/status/1478401918792011776?s=20&t=AVPOmNLtaozrA0Ccp6DyAw
        cupidatat excepteur officia. Reprehenderit nostrud nostrud ipsum Lorem est aliquip amet
        https://b23.tv/C0lw13z
        voluptate voluptate dolor minim nulla est proident. Nostrud officia pariatur ut officia.
        https://www.amazon.com/b/?node=226184&ref_=Oct_d_odnav_d_1077068_1&pd_rd_w=ZjwFQ&pf_rd_p=0f6f8a08-29ea-497e-8cb4-0ccf91422740&pf_rd_r=YMQ5XPAZHYHV77HCENY7&pd_rd_r=27c502f2-951f-4a8c-9478-381febc5e5bc&pd_rd_wg=NxaQ1
        Sit irure elit esse ea nulla sunt ex occaecat reprehenderit commodo officia dolor Lorem
        https://post.m.smzdm.com/p/aoxzv08r/?zdm_ss=iOS__hczZ7LgGInW%2BUXtAcwyZGSVdJqcPFvT98aEipRx9K%2BPOH7mQ0YGD3w%3D%3D&from=other
        duis laboris cupidatat officia voluptate. Culpa proident adipisicing id nulla nisi laboris
        https://example.com?utm_source=ios
        ex in Lorem sunt duis officia eiusmod. Aliqua reprehenderit commodo ex non excepteur duis
        sunt velit enim. Voluptate laboris sint cupidatat ullamco ut ea consectetur et est culpa
        et culpa duis.";

    let expect = "Lorem ipsum dolor sit amet, officia excepteur ex fugiat reprehenderit enim
        labore culpa sint ad nisi Lorem pariatur mollit ex esse exercitation amet. Nisi anim
        https://twitter.com/CiloRanko/status/1478401918792011776
        cupidatat excepteur officia. Reprehenderit nostrud nostrud ipsum Lorem est aliquip amet
        https://www.bilibili.com/video/BV1GJ411x7h7?p=1
        voluptate voluptate dolor minim nulla est proident. Nostrud officia pariatur ut officia.
        https://www.amazon.com/b/?node=226184
        Sit irure elit esse ea nulla sunt ex occaecat reprehenderit commodo officia dolor Lorem
        https://post.m.smzdm.com/p/aoxzv08r/
        duis laboris cupidatat officia voluptate. Culpa proident adipisicing id nulla nisi laboris
        https://example.com/
        ex in Lorem sunt duis officia eiusmod. Aliqua reprehenderit commodo ex non excepteur duis
        sunt velit enim. Voluptate laboris sint cupidatat ullamco ut ea consectetur et est culpa
        et culpa duis.";

    let cleaner = Arc::new(clearurl::UrlCleaner::from_file("../rulesV1.toml").unwrap());
    let get = replace(original, &cleaner).unwrap();

    assert_eq!(get, expect);
}

#[tokio::test]
async fn test_clean() {
    let cleaner = clearurl::UrlCleaner::from_file("../rules.toml").unwrap();
    let cleaner = Arc::new(cleaner);

    // rick roll
    let input = "https://www.bilibili.com/video/av928861104";
    let link = clean(input, &cleaner).await.unwrap();

    // it should return nothing
    assert!(link.data.is_empty());
    assert_eq!(link.met, 1);
    assert_eq!(link.cleaned, 0);

    let input = "https://b23.tv/YfzhsWH";
    let link = clean(input, &cleaner).await.unwrap();

    // It should return expected string
    assert!(!link.data.is_empty());
    assert_eq!(link.met, 1);
    assert_eq!(link.cleaned, 1);
    assert_eq!(
        link.data,
        vec![url::Url::parse("https://www.bilibili.com/video/BV1vZ4y1Z7Y7?p=1").unwrap()]
    );

    let input =
        "https://twitter.com/USAO926/status/1531171681792065536?s=20&t=lsssIcZ7sY8IAwbhAO1d2g";
    let link = clean(input, &cleaner).await.unwrap();

    // It should return expected vxtwitter
    assert!(!link.data.is_empty());
    assert_eq!(link.met, 1);
    assert_eq!(link.cleaned, 1);
    assert_eq!(
        link.data,
        vec![url::Url::parse("https://vxtwitter.com/USAO926/status/1531171681792065536").unwrap()]
    );
}
