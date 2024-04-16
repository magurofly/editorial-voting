use actix_web;
use serde;

use crate::atcoder_api;

#[derive(serde::Deserialize)]
struct Req {
    token: Option<String>,
    editorial: String,
    vote: String,
}

#[derive(serde::Serialize)]
struct Res {
    status: &'static str,
    reason: Option<String>,
}

#[actix_web::post("/create-affiliation-token")]
pub async fn route(req: actix_web::web::Json<Req>) -> actix_web::HttpResponse {
    todo!()
}
