use std::collections::HashMap;

use lazy_static::lazy_static;

type HookFn = fn(input: &url::Url) -> anyhow::Result<url::Url>;

lazy_static! {
    pub static ref POST_HOOKS: HashMap<String, HookFn> = HashMap::from([
            #[cfg(feature = "bilibili_hooks")]
            ("bv_to_av".to_string(), bv_to_av as HookFn),
            ("fixup_twitter".to_string(), fixup_twitter as HookFn)
        ]);

    // Internal
    static ref TRANSLATE: HashMap<char, u64> = {
        TABLE
            .chars()
            .enumerate()
            .map(|(i, c)| (c, i as u64))
            .collect()
    };
}

const TABLE: &str = "fZodR9XQDSUm21yCkr6zBqiveYah8bt4xsWpHnJE7jL5VG3guMTKNPAwcF";
#[cfg(feature = "bilibili_hooks")]
const SELECT: [usize; 6] = [11, 10, 3, 8, 4, 6];
#[cfg(feature = "bilibili_hooks")]
const XOR: u64 = 177451812;
#[cfg(feature = "bilibili_hooks")]
const ADD: u64 = 8728348608;

#[cfg(feature = "bilibili_hooks")]
fn bv_to_av(input: &url::Url) -> anyhow::Result<url::Url> {
    if input.domain().is_none() {
        anyhow::bail!("domain is empty");
    }

    if input.path_segments().is_none() {
        anyhow::bail!("url doesn't have path segment");
    }

    let segments: Vec<_> = input.path_segments().unwrap().collect();
    if segments.len() < 2 {
        anyhow::bail!("path segment is too short: {input}");
    }
    if segments[0] != "video" {
        anyhow::bail!("{input} is not a video URL");
    }
    if !segments[1].starts_with("BV") || !segments.len() == 12 {
        anyhow::bail!("{input} is not a valid BV-encoded video URL");
    }

    let chars: Vec<char> = segments[1].chars().collect();
    let result: u64 = (0..6).fold(0, |acc, i| {
        let select = SELECT[i];
        let char = chars[select];
        let translated = TRANSLATE[&char];
        acc + translated * (58_u64.pow(i as u32))
    });
    let avid = (result - ADD) ^ XOR;

    let mut new_url = input.clone();
    new_url
        .path_segments_mut()
        .unwrap()
        .clear()
        .extend([segments[0], &format!("av{avid}"), ""]);

    Ok(new_url)
}

#[cfg(feature = "bilibili_hooks")]
#[test]
fn test_bv_to_av() {
    let a = url::Url::parse("https://www.bilibili.com/video/BV1nY411r7o1/?p=1").unwrap();
    assert_eq!(
        bv_to_av(&a).unwrap().to_string(),
        "https://www.bilibili.com/video/av267692137/?p=1"
    );
    let b = url::Url::parse("https://www.bilibili.com/video/av747880465?p=1").unwrap();
    assert!(bv_to_av(&b).is_err());
}

fn fixup_twitter(input: &url::Url) -> anyhow::Result<url::Url> {
    if input.domain().is_none() {
        anyhow::bail!("domain is empty");
    }

    let domain = input.domain().unwrap();
    let fixup_domain = match domain {
        "twitter.com" => "fxtwitter.com",
        "x.com" => "fixupx.com",
        _ => anyhow::bail!("not a valid twitter URL"),
    };
    let mut new_url = input.clone();
    new_url.set_host(Some(fixup_domain)).unwrap();
    Ok(new_url)
}
