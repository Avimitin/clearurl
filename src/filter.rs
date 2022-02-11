use crate::data::Domains;
use anyhow::{bail, Context, Result};
use url::form_urlencoded;
use url::Url;

pub async fn clear(domains: &Domains, url: &mut Url) -> Result<Url> {
    remove_query(domains, url).await
}

#[async_recursion::async_recursion]
async fn remove_query(domains: &Domains, url: &mut Url) -> Result<Url> {
    // get domain from url
    let domain = url.domain();
    if domain.is_none() {
        bail!("invalid url: {}", url.as_str())
    }
    let domain = domain.unwrap();

    // get rule by domain
    let mut domain_rule = domains
        .get(domain)
        .context(format!("no rule for domain: <{}>", domain))?;

    // if the domain require redirect
    if domain_rule.should_redirect {
        let mut final_url = get_final_url(url.as_str()).await.context(format!(
            "fail to make redirection for domain {}",
            url.as_str()
        ))?;
        return remove_query(domains, &mut final_url).await;
    }

    if domain_rule.rules.is_empty() {
        bail!("no rule for domain: <{}>", domain)
    }

    let blacklist = &domain_rule.rules;

    // take a copy of the query string for later use
    let ori_queries = url.query();
    // if no query behind, return domain back immediately
    if ori_queries.is_none() {
        return Ok(url.to_owned());
    }
    // get the copy of the queries
    let ori_queries = ori_queries.unwrap().to_string();

    // and parse it into pairs -> [(k, v)]
    let ori_queries = form_urlencoded::parse(ori_queries.as_bytes());

    // clean the original queries to get a clean url
    url.set_query(None);

    // append queries that are not in the blacklist
    for (key, val) in ori_queries {
        let mut has_same = false;
        for query in blacklist {
            if key == query.as_str() {
                has_same = true;
                break;
            }
        }
        if !has_same {
            url.query_pairs_mut().append_pair(&key, &val);
        }
    }

    Ok(url.to_owned())
}

async fn get_final_url(url: &str) -> Result<Url> {
    let resp = reqwest::get(url).await?;

    Ok(resp.url().to_owned())
}

#[tokio::test]
async fn test_filter() {
    let data =
        crate::data::Domains::load_from_file("./rules.toml").expect("fail to read rules.toml");

    // * test normal rule
    let mut url = Url::parse(
        "https://twitter.com/CiloRanko/status/1478401918792011776?s=20&t=AVPOmNLtaozrA0Ccp6DyAw",
    )
    .unwrap();
    let url = clear(&data, &mut url).await.unwrap();
    assert_eq!(
        url.as_str(),
        "https://twitter.com/CiloRanko/status/1478401918792011776"
    );

    // * test redirection
    let mut url = Url::parse("https://b23.tv/C0lw13z").unwrap();
    let url = clear(&data, &mut url).await.unwrap();
    assert_eq!(
        url.as_str(),
        // normal queries will be kept
        "https://www.bilibili.com/video/BV1GJ411x7h7?p=1"
    );
}
