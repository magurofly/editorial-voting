use sha2::{Digest, Sha256};

const ATCODER_ID_PATTERN: regex::Regex = regex::Regex::new(r#"^[0-9A-Za-z]{3,16}$"#).unwrap();
const AFFILIATION_TOKEN_PATTERN: regex::Regex = regex::Regex::new(r#"^[0-9a-f]{16}-[0-9a-f]{64}$"#).unwrap();
const TOKEN_PATTERN: regex::Regex = regex::Regex::new(r#"^[0-9a-f]{16}-[0-9A-Za-z]{3,16}-[0-9a-f]{64}$"#).unwrap();
static CONTEST_PATTERN: regex::Regex = regex::Regex::new(r#"^[-\w]+$"#).unwrap();

pub fn validate_atcoder_id(atcoder_id: &str) -> bool {
    ATCODER_ID_PATTERN.is_match(atcoder_id)
}

fn token_hash(time: u64, atcoder_id: &str, salt: &str) -> String {
    let time_str = format!("{time:016x}");
    let mut plaintext = String::new();
    plaintext.push_str(&time_str);
    plaintext.push(':');
    plaintext.push_str(atcoder_id);
    plaintext.push(':');
    plaintext.push_str(salt);
    hex::encode(&Sha256::digest(&plaintext.into_bytes()))
}

pub fn create_affiliation_token(time_sec: u64, atcoder_id: &str) -> String {
    let salt = std::env::var("EDITORIAL_VOTING_AFFILIATION_TOKEN_SALT").unwrap();
    let mut affiliation_token = String::new();
    affiliation_token.push_str(&format!("{time_sec:x}"));
    affiliation_token.push_str(&token_hash(time_sec, atcoder_id, &salt));
    affiliation_token
}

pub fn validate_affiliation_token(atcoder_id: &str, affiliation_token: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !validate_atcoder_id(atcoder_id) || !AFFILIATION_TOKEN_PATTERN.is_match(affiliation_token) {
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
    let hash = token_hash(time_sec, atcoder_id, &salt);
    if hash != hash_orig {
        return Err("invalid affiliation_token".into());
    }
    Ok(())
}

pub fn create_token(time_sec: u64, atcoder_id: &str) -> String {
    let salt = std::env::var("EDITORIAL_VOTING_TOKEN_SALT").unwrap();
    let mut token = String::new();
    token.push_str(&format!("{time_sec:x}"));
    token.push('-');
    token.push_str(atcoder_id);
    token.push('-');
    token.push_str(&token_hash(time_sec, atcoder_id, &salt));
    token
}

pub fn parse_token(token: &str) -> Result<String, Box<dyn std::error::Error>> {
    if !AFFILIATION_TOKEN_PATTERN.is_match(token) {
        return Err("affiliation_token invalid format".into());
    }
    let salt = std::env::var("EDITORIAL_VOTING_TOKEN_SALT")?;
    let mut split = token.split("-");
    let time_str = split.next().unwrap();
    let atcoder_id = split.next().unwrap();
    let hash_orig = split.next().unwrap();
    let time_sec = u64::from_str_radix(&time_str, 16)?;
    let hash = token_hash(time_sec, atcoder_id, &salt);
    if hash != hash_orig {
        return Err("invalid token".into());
    }
    Ok(atcoder_id.to_string())
}

pub async fn scrape_affiliation(atcoder_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    if !validate_atcoder_id(atcoder_id) {
        return Err("invalid atcoder id".into());
    }
    let res = awc::Client::default().get(format!("https://atcoder.jp/users/{atcoder_id}")).send().await?.body().await?;
    let doc = scraper::Html::parse_document(&std::str::from_utf8(&res)?);
    let selector = scraper::Selector::parse("#main-container > div.row > div.col-md-3.col-sm-12 > table > tbody > tr:nth-child(5) > td")?;
    let affiliation_element = doc.select(&selector).next().ok_or_else::<String, _>(|| "affiliation not found".into() )?;
    Ok(affiliation_element.text().collect::<Vec<_>>().join(""))
}

pub async fn scrape_editorials(contest: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    if !CONTEST_PATTERN.is_match(&contest) {
        return Err("task invalid format".into());
    }

    let res = awc::Client::default().get(format!("https://atcoder.jp/contests/{contest}/editorial")).send().await?.body().await?;
    let doc = scraper::Html::parse_document(&std::str::from_utf8(&res)?);
    let selector = scraper::Selector::parse("#main-container > div.row > div:nth-child(2) li > a[rel=noopener]")?;
    let editorials = doc.select(&selector).filter_map(|link| link.attr("href").and_then(|url| canonicalize_editorial_url(url) ) ).collect::<Vec<_>>();
    Ok(editorials)
}

fn canonicalize_editorial_url(url: &str) -> Option<String> {
    if url.starts_with("/jump?url=") {
        let encoded = url.split_at(10).1;
        return Some(urlencoding::decode(encoded).expect("UTF-8").to_string());
    }

    if url.starts_with("/contests/") {
        let mut full = String::new();
        full.push_str("https://atcoder.jp");
        full.push_str(url);
        return Some(full);
    }

    None
}