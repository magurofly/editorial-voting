use sha2::{Digest, Sha256};

pub fn validate_atcoder_id(atcoder_id: &str) -> bool {
    regex::Regex::new(r#"^[0-9A-Za-z]{3,16}$"#).unwrap().is_match(atcoder_id)
}

fn affiliation_token_hash(time_sec: u64, atcoder_id: &str, salt: &str) -> String {
    let mut plaintext = String::new();
    plaintext.push_str(&format!("{time_sec:016x}"));
    plaintext.push(':');
    plaintext.push_str(atcoder_id);
    plaintext.push(':');
    plaintext.push_str(salt);
    hex::encode(&Sha256::digest(&plaintext.into_bytes()))
}

pub fn create_affiliation_token(time_sec: u64, atcoder_id: &str) -> String {
    let salt = std::env::var("EDITORIAL_VOTING_AFFILIATION_TOKEN_SALT").unwrap();
    let mut affiliation_token = String::new();
    affiliation_token.push_str(&format!("{time_sec:016x}"));
    affiliation_token.push('-');
    affiliation_token.push_str(&affiliation_token_hash(time_sec, atcoder_id, &salt));
    affiliation_token
}

pub fn validate_affiliation_token(atcoder_id: &str, affiliation_token: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !validate_atcoder_id(atcoder_id) || !regex::Regex::new(r#"^[0-9a-f]{16}-[0-9a-f]{64}$"#).unwrap().is_match(affiliation_token) {
        return Err("affiliation_token invalid format".into());
    }
    let salt = std::env::var("EDITORIAL_VOTING_AFFILIATION_TOKEN_SALT")?;
    let mut split = affiliation_token.split("-");
    let time_str = split.next().unwrap();
    let hash_orig = split.next().unwrap();
    let time_sec = u64::from_str_radix(&time_str, 16)?;
    let created_time = std::time::SystemTime::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(time_sec)).ok_or_else::<String, _>(|| "affiliation_token invalid time".into() )?;
    let current_time = std::time::SystemTime::now();
    if current_time.duration_since(created_time)? > std::time::Duration::from_secs(60 * 60) {
        return Err("affiliation_token expired".into());
    }
    let hash = affiliation_token_hash(time_sec, atcoder_id, &salt);
    if hash != hash_orig {
        return Err("invalid affiliation_token".into());
    }
    Ok(())
}

fn token_hash(time_sec: u64, atcoder_id: &str, user_id: usize, salt: &str) -> String {
    let mut plaintext = String::new();
    plaintext.push_str(&format!("{time_sec:016x}"));
    plaintext.push(':');
    plaintext.push_str(atcoder_id);
    plaintext.push(':');
    plaintext.push_str(&user_id.to_string());
    plaintext.push(':');
    plaintext.push_str(salt);
    hex::encode(&Sha256::digest(&plaintext.into_bytes()))
}

pub fn create_token(time_sec: u64, atcoder_id: &str, user_id: usize) -> String {
    let salt = std::env::var("EDITORIAL_VOTING_TOKEN_SALT").unwrap();
    let mut token = String::new();
    token.push_str(&format!("{time_sec:016x}"));
    token.push('-');
    token.push_str(atcoder_id);
    token.push('-');
    token.push_str(&user_id.to_string());
    token.push('-');
    token.push_str(&token_hash(time_sec, atcoder_id, user_id, &salt));
    token
}

pub struct UserToken {
    pub atcoder_id: String,
    pub user_id: usize,
    pub time_created: u64,
}

pub fn parse_token(token: &str) -> Result<UserToken, Box<dyn std::error::Error>> {
    if !regex::Regex::new(r#"^[0-9a-f]{16}-[0-9A-Za-z]{3,16}-[0-9]+-[0-9a-f]{64}$"#).unwrap().is_match(token) {
        return Err("token invalid format".into());
    }
    let salt = std::env::var("EDITORIAL_VOTING_TOKEN_SALT")?;
    let mut split = token.split("-");
    let time_str = split.next().unwrap();
    let atcoder_id = split.next().unwrap();
    let user_id = usize::from_str_radix(split.next().unwrap(), 10)?;
    let hash_orig = split.next().unwrap();
    let time_sec = u64::from_str_radix(&time_str, 16)?;
    let hash = token_hash(time_sec, atcoder_id, user_id, &salt);
    if hash != hash_orig {
        return Err("invalid token".into());
    }
    Ok(UserToken {
        atcoder_id: atcoder_id.to_string(),
        user_id,
        time_created: time_sec,
    })
}

pub async fn scrape_affiliation(atcoder_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    if !validate_atcoder_id(atcoder_id) {
        return Err("invalid atcoder id".into());
    }
    let res = awc::Client::default().get(format!("https://atcoder.jp/users/{atcoder_id}?lang=en")).send().await?.body().await?;
    let doc = scraper::Html::parse_document(&std::str::from_utf8(&res)?);
    let selector = scraper::Selector::parse("#main-container > div.row > div.col-md-3.col-sm-12 > table > tbody > tr")?;
    let affiliation = doc.select(&selector)
        .filter_map(|row| {
            let mut children = row.child_elements();
            if children.next()?.text().next()? == "Affiliation" {
                children.next()?.text().next()
            } else {
                None
            }
        })
        .next()
        .ok_or_else::<String, _>(|| "affiliation not found".into() )?;
    Ok(affiliation.to_string())
}

pub async fn scrape_editorials(contest: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    if !regex::Regex::new(r#"^[-\w]+$"#).unwrap().is_match(&contest) {
        return Err("contest invalid format".into());
    }
    let mut editorials = vec![];
    for lang in &["ja", "en"] {
        let res = awc::Client::default().get(format!("https://atcoder.jp/contests/{contest}/editorial?editorialLang={lang}")).send().await?.body().await?;
        let doc = scraper::Html::parse_document(&std::str::from_utf8(&res)?);
        let selector = scraper::Selector::parse(r#"#main-container a[rel="noopener"]"#)?;
        editorials.extend(doc.select(&selector).filter_map(|link| link.attr("href").and_then(|url| canonicalize_editorial_url(url) ) ));
    }
    Ok(editorials)
}

fn canonicalize_editorial_url(url: &str) -> Option<String> {
    if url.starts_with("/jump?url=") {
        let encoded = url.split_at(10).1;
        return Some(urlencoding::decode(encoded).expect("UTF-8").to_string());
    }

    if url.starts_with("/") {
        let mut full = String::new();
        full.push_str("https://atcoder.jp");
        full.push_str(url);
        return Some(full);
    }

    Some(url.to_string())
}

pub struct AtCoderUserDetails {
    pub rating: i64,
}
pub async fn scrape_user(atcoder_id: &str) -> Result<AtCoderUserDetails, Box<dyn std::error::Error>> {
    if !validate_atcoder_id(atcoder_id) {
        return Err("invalid atcoder id".into());
    }

    let res = awc::Client::default().get(format!("https://atcoder.jp/users/{atcoder_id}")).send().await?.body().await?;
    let doc = scraper::Html::parse_document(&std::str::from_utf8(&res)?);
    let selector = scraper::Selector::parse("#main-container > div.row > div.col-md-9.col-sm-12 > table > tbody > tr:nth-child(2) > td > span")?;
    let rating = doc.select(&selector).next().and_then(|elem| elem.text().next() ).and_then(|rating| i64::from_str_radix(rating, 10).ok() ).unwrap_or(0);

    Ok(AtCoderUserDetails {
        rating,
    })
}