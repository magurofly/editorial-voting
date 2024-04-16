use std::sync::Mutex;

use actix_web;
use serde;

use crate::atcoder_api;

#[derive(serde::Deserialize)]
struct Req {
    token: Option<String>,
    contest: String,
    editorial: String,
}

#[derive(Default, serde::Serialize)]
struct Res {
    status: &'static str,
    reason: Option<String>,
    score: Option<i64>,
    current_vote: Option<&'static str>,
}

#[actix_web::post("/create-affiliation-token")]
pub async fn route(req: actix_web::web::Json<Req>, data: actix_web::web::Data<Mutex<rusqlite::Connection>>) -> actix_web::HttpResponse {
    // let conn = data.lock().unwrap();

    // let editorial_id = {
    //     if let Ok(id) = conn.query_row("SELECT id FROM editorials WHERE editorial = ?1", [req.editorial.clone()], |row| row.get::<_, usize>(0) ) {
    //         Ok(id)
    //     } else {
    //         for editorial in atcoder_api::scrape_editorials(&req.contest).await? {

    //         }
    //     }
    // }

    todo!()
}
