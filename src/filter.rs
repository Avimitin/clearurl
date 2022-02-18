use crate::data::{DomainConfig, RulesStorage};
use anyhow::{anyhow, bail, Context, Result};
use regex::Regex;
use url::form_urlencoded;
use url::Url;

pub async fn clear(rulestore: &RulesStorage, url: &str) -> Result<Url> {
    // The variable `purl` stands for parsed url. I need the original url value for bug tracking.
    // So I use a new variable not shadow the original `url` variable here.
    let mut purl = Url::parse(url)?;

    // if there is no queries in url, return it immediately
    if purl.query().is_none() {
        return Ok(purl);
    }

    // We need a url copy to have this domain is mutable during runtime
    let mut domain = purl
        .domain()
        .ok_or_else(|| anyhow!("no domain for url {}", url))?
        .to_owned();
    let mut domain_rule = rulestore
        .get(&domain)
        .context(format!("get pre-set rule for domain: {}", url))?;

    // if the domain rule should be redirect, get the final url and ruleset
    if domain_rule.should_redirect {
        purl = get_final_url(url)
            .await
            .context(format!("redirect from domain {}", url))?;
        domain = purl
            .domain()
            .ok_or_else(|| anyhow!("fail to parse url {} redirect from: {}", purl.as_str(), url))?
            .to_owned();

        domain_rule = rulestore.get(&domain).context(format!(
            "get pre-set rule for domain: {} redirect from: {}",
            purl.as_str(),
            url
        ))?;
    }

    // if the domain need to import from other domain and not import yet
    let import_rule = if domain_rule.has_import() {
        Some(rulestore.get(&domain_rule.import).context(format!(
            "domain {} import data from {}",
            domain, domain_rule.import
        ))?)
    } else {
        None
    };

    // If the domain still have no rule after import, it means that no rules
    // is declare in the rules.toml file.
    if !domain_rule.has_rules() && !domain_rule.has_import() {
        // it is safe to use unwrap because we already handle the `None` value.
        bail!("no rule for domain: <{}>", purl.domain().unwrap())
    }

    // finally remove all the queries
    remove_query(&mut purl, domain_rule, import_rule).await
}

async fn remove_query(
    url: &mut Url,
    domain_rule: &DomainConfig,
    import_rule: Option<&DomainConfig>,
) -> Result<Url> {
    let blacklist = &domain_rule.rules;
    let imp_blacklist = if let Some(imp_rule) = import_rule {
        Some(&imp_rule.rules)
    } else {
        None
    };

    // Take a copy of the query string for later use.
    // It is safe to call unwrap here cuz we had handle `None` in the
    // `filter::clear()` function.
    let ori_queries = url.query().unwrap().to_owned();

    // and parse it into pairs -> [(k, v)]
    let ori_queries = form_urlencoded::parse(ori_queries.as_bytes());

    // clean the original queries to get a clean url
    url.set_query(None);

    // append queries that are not in the blacklist
    for (key, val) in ori_queries {
        let mut has_same = false;
        for query in blacklist {
            let re = Regex::new(query.as_str())
                .context(format!("illgal regex rule: {}", query.as_str()))?;
            if re.is_match(&key) {
                has_same = true;
                break;
            }
        }

        if let Some(query) = imp_blacklist {
            for q in query {
                let re = Regex::new(q.as_str()).context(format!(
                    "illgal regex rule '{}' in {}'s import",
                    q.as_str(),
                    url
                ))?;
                if re.is_match(&key) {
                    has_same = true;
                    break;
                }
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
