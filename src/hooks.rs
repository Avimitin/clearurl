use std::collections::HashMap;

use lazy_static::lazy_static;

type HookFn = fn(input: &url::Url) -> anyhow::Result<url::Url>;

lazy_static! {
    pub static ref POST_HOOKS: HashMap<String, HookFn> =
        HashMap::from([("bv_to_av".to_string(), bv_to_av as HookFn)]);
    static ref TRANSLATE: HashMap<char, u64> = {
        TABLE
            .chars()
            .enumerate()
            .map(|(i, c)| (c, i as u64))
            .collect()
    };
}

const TABLE: &str = "fZodR9XQDSUm21yCkr6zBqiveYah8bt4xsWpHnJE7jL5VG3guMTKNPAwcF";
const SELECT: [usize; 6] = [11, 10, 3, 8, 4, 6];
const XOR: u64 = 177451812;
const ADD: u64 = 8728348608;

fn bv_to_av(input: &url::Url) -> anyhow::Result<url::Url> {
    if input.domain().is_none() {
        anyhow::bail!("domain is empty");
    }

    if input.path_segments().is_none() {
        anyhow::bail!("url doesn't have path segment");
    }

    let segments: Vec<_> = input.path_segments().unwrap().collect();
    if segments.len() < 2 {
        anyhow::bail!("not a valid bilibili video URL: path segment is too short");
    }
    if segments[0] != "video" {
        anyhow::bail!("not a valid bilibili video URL: not a video URL");
    }
    if !segments[1].starts_with("BV") && !segments.len() == 12 {
        anyhow::bail!("not a valid bilibili video URL: not a valid BV-encoded video URL");
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

#[test]
fn test_bv_to_av() {
    let a = url::Url::parse("https://www.bilibili.com/video/BV1nY411r7o1/?p=1").unwrap();
    assert_eq!(
        bv_to_av(&a).unwrap().to_string(),
        "https://www.bilibili.com/video/av267692137/?p=1"
    );
}
