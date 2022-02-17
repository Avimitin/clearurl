use crate::data::{RulesStorage, DomainConfig};
use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;
use url::form_urlencoded;
use url::Url;

pub async fn clear(rulestore: &RulesStorage, url: &str) -> Result<Url> {
    // The variable `purl` stands for parsed url. I need the original url value for bug tracking.
    // So I use a new variable not shadow the original `url` variable here.
    let mut purl = Url::parse(url)?;

    // check if the url is valid
    let domain = purl
        .domain()
        .ok_or_else(|| anyhow!("fail to parse url {}", url))?;
    let mut domain_rule = rulestore
        .get(domain)
        .context(format!("get pre-set rule for domain: {}", url))?;

    // if the domain rule should be redirect, get the final url and ruleset
    if domain_rule.should_redirect {
        purl = get_final_url(url)
            .await
            .context(format!("redirect from domain {}", url))?;
        let final_domain = purl
            .domain()
            .ok_or_else(|| anyhow!("fail to parse url {} redirect from: {}", purl.as_str(), url))?;

        domain_rule = rulestore.get(final_domain).context(format!(
            "get pre-set rule for domain: {} redirect from: {}",
            purl.as_str(),
            url
        ))?;
    }

    // if the domain need to import from other domain and not import yet
    if domain_rule.has_import() && !domain_rule.is_imported() {
        unimplemented!("implement rule import")
    }

    // If the domain still have no rule after import, it means that no rules
    // is declare in the rules.toml file.
    if !domain_rule.has_rules() {
        // it is safe to use unwrap because we already handle the `None` value.
        bail!("no rule for domain: <{}>", purl.domain().unwrap())
    }

    // finally remove all the queries
    remove_query(domain_rule, &mut purl).await
}

async fn remove_query(domain_rule: &DomainConfig, url: &mut Url) -> Result<Url> {
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
            let re = Regex::new(query.as_str()).unwrap();
            if re.is_match(&key) {
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
    let data = RulesStorage::load_from_file("./rules.toml").expect("fail to read rules.toml");

    // * test normal rule
    let url = clear(
        &data,
        "https://twitter.com/CiloRanko/status/1478401918792011776?s=20&t=AVPOmNLtaozrA0Ccp6DyAw",
    )
    .await
    .unwrap();
    assert_eq!(
        url.as_str(),
        "https://twitter.com/CiloRanko/status/1478401918792011776"
    );

    // * test redirection
    let url = clear(&data, "https://b23.tv/C0lw13z").await.unwrap();
    assert_eq!(
        url.as_str(),
        // normal queries will be kept
        "https://www.bilibili.com/video/BV1GJ411x7h7?p=1"
    );

    // * test regex
    let url = clear(
        &data,
        "https://www.amazon.com/b/?node=226184&ref_=Oct_d_odnav_d_1077068_1&pd_rd_w=ZjwFQ&pf_rd_p=0f6f8a08-29ea-497e-8cb4-0ccf91422740&pf_rd_r=YMQ5XPAZHYHV77HCENY7&pd_rd_r=27c502f2-951f-4a8c-9478-381febc5e5bc&pd_rd_wg=NxaQ1"
    )
    .await
    .unwrap();
    assert_eq!(
        url.as_str(),
        // normal queries will be kept
        "https://www.amazon.com/b/?node=226184"
    );
}
// vim: tw=80 fo+=t
