use crate::rules;
use url::{form_urlencoded, Url};

pub async fn clear(url: &str, store: &rules::RuntimeRules) -> anyhow::Result<reqwest::Url> {
    let mut url = Url::parse(url)?;

    let get_rule = |url: &reqwest::Url| -> anyhow::Result<&rules::Rule> {
        let domain = url
            .domain()
            .ok_or_else(|| anyhow::anyhow!("no domain"))?
            .to_string();
        match store.get(&domain) {
            Some(r) => Ok(r),
            None => Ok(store
                .get("default")
                .ok_or_else(|| anyhow::anyhow!("no default rule was found"))?),
        }
    };

    let mut rule = get_rule(&url)?;

    if rule.redirect {
        url = reqwest::get(url).await?.url().clone();
        rule = get_rule(&url)?;
    }

    if url.query().is_none() {
        anyhow::bail!("Empty query")
    }

    let query = url.query().unwrap();
    if query.is_empty() {
        anyhow::bail!("Empty query")
    }

    if rule.rules.is_empty() {
        anyhow::bail!("Empty rules")
    }

    // Copy the original query and parse it into pairs
    let query = query.to_string();
    let query = form_urlencoded::parse(query.as_bytes());

    // clean the original one
    url.set_query(None);

    for (k, v) in query {
        let mut met = false;
        for re in &rule.rules {
            if re.is_match(&k) {
                met = true;
                break;
            }
        }

        if !met {
            url.set_query(Some(&format!("{k}={v}")))
        }
    }

    Ok(url)
}

#[tokio::test]
async fn test_filter() {
    let data = rules::parse(std::path::Path::new("./rulesV1.toml"));

    // * test normal rule
    let url = clear(
        "https://twitter.com/CiloRanko/status/1478401918792011776?s=20&t=AVPOmNLtaozrA0Ccp6DyAw",
        &data,
    )
    .await
    .unwrap();
    assert_eq!(
        url.as_str(),
        "https://twitter.com/CiloRanko/status/1478401918792011776"
    );

    // * test redirection
    let url = clear("https://b23.tv/C0lw13z", &data).await.unwrap();
    assert_eq!(
        url.as_str(),
        // normal queries will be kept
        "https://www.bilibili.com/video/BV1GJ411x7h7?p=1"
    );

    // * test regex
    let url = clear(
        "https://www.amazon.com/b/?node=226184&ref_=Oct_d_odnav_d_1077068_1&pd_rd_w=ZjwFQ&pf_rd_p=0f6f8a08-29ea-497e-8cb4-0ccf91422740&pf_rd_r=YMQ5XPAZHYHV77HCENY7&pd_rd_r=27c502f2-951f-4a8c-9478-381febc5e5bc&pd_rd_wg=NxaQ1",
        &data,
    )
    .await
    .unwrap();
    assert_eq!(
        url.as_str(),
        // normal queries will be kept
        "https://www.amazon.com/b/?node=226184"
    );

    // * test import
    let url = clear(
        "https://post.m.smzdm.com/p/aoxzv08r/?zdm_ss=iOS__hczZ7LgGInW%2BUXtAcwyZGSVdJqcPFvT98aEipRx9K%2BPOH7mQ0YGD3w%3D%3D&from=other",
        &data,
    )
    .await
    .unwrap();
    assert_eq!(
        url.as_str(),
        // normal queries will be kept
        "https://post.m.smzdm.com/p/aoxzv08r/",
    );

    // * test default
    let url = clear("https://example.com?utm_source=ios", &data)
        .await
        .unwrap();
    assert_eq!(
        url.as_str(),
        // normal queries will be kept
        "https://example.com/",
    );
}
// vim: tw=80 fo+=t
