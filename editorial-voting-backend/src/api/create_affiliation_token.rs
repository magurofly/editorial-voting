use actix_web;
use serde;

use crate::atcoder_api;

#[derive(serde::Deserialize)]
struct Req {
    atcoder_id: String,
}

#[derive(serde::Serialize)]
struct Res {
    status: &'static str,
    reason: Option<String>,
    affiliation_token: Option<String>,
}

#[actix_web::post("/create-affiliation-token")]
pub async fn route(req: actix_web::web::Json<Req>) -> actix_web::HttpResponse {
    if !atcoder_api::validate_atcoder_id(&req.atcoder_id) {
        return actix_web::HttpResponse::Ok().json(Res {
            status: "error",
            reason: Some("invalid atcoder_id format".to_string()),
            affiliation_token: None,
        });
    }

    let time = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs();
    let affiliation_token = atcoder_api::create_affiliation_token(time, &req.atcoder_id);
    
    actix_web::HttpResponse::Ok().json(Res {
        status: "success",
        reason: None,
        affiliation_token: Some(affiliation_token),
    })
}
