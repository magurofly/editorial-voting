use std::sync::Mutex;

use actix_web::{self, HttpResponse};
use serde;

use crate::atcoder_api;

#[derive(serde::Deserialize)]
struct Req {
    atcoder_id: String,
    affiliation_token: String,
}

#[derive(Default, Clone, serde::Serialize)]
struct Res {
    status: &'static str,
    reason: Option<String>,
    token: Option<String>,
}

async fn inner(req: &Req, data: actix_web::web::Data<Mutex<rusqlite::Connection>>) -> Result<Res, Box<dyn std::error::Error>> {
    let time = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs();

    atcoder_api::validate_affiliation_token(&req.atcoder_id, &req.affiliation_token)?;

    let affiliation_token_get = atcoder_api::scrape_affiliation(&req.atcoder_id).await?;
    
    if req.affiliation_token != affiliation_token_get {
        return Err("affiliation token not matched".into());
    }

    let user_id = {
        let conn = data.lock().unwrap();
        // add user if not exist
        conn.execute("INSERT OR IGNORE INTO users(atcoder_id) VALUES(?1)", [&req.atcoder_id])?;
        // get user id
        conn.query_row("SELECT id FROM users WHERE atcoder_id = ?1", [&req.atcoder_id], |row| row.get::<_, usize>(0) )?
    };

    let token = atcoder_api::create_token(time, &req.atcoder_id, user_id);

    Ok(Res {
        status: "success",
        reason: None,
        token: Some(token),
    })
}

#[actix_web::post("/create-token")]
pub async fn route(req: actix_web::web::Json<Req>, data: actix_web::web::Data<Mutex<rusqlite::Connection>>) -> actix_web::HttpResponse {

    match inner(&req, data).await {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(reason) => HttpResponse::Ok().json(Res {
            status: "error",
            reason: Some(reason.to_string()),
            ..Res::default()
        }),
    }
}