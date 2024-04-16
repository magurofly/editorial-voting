use actix_web;
use serde;

use crate::atcoder_api;

#[derive(serde::Deserialize)]
struct Req {
    atcoder_id: String,
    affiliation_token: String,
}

#[derive(Clone, serde::Serialize)]
struct Res {
    status: &'static str,
    reason: Option<String>,
    token: Option<String>,
}

#[actix_web::post("/create-token")]
pub async fn route(req: actix_web::web::Json<Req>) -> actix_web::HttpResponse {
    if let Err(reason) = atcoder_api::validate_affiliation_token(&req.atcoder_id, &req.affiliation_token) {
        return actix_web::HttpResponse::Ok().json(Res {
            status: "error",
            reason: Some(reason.to_string()),
            token: None,
        });
    }

    let affiliation_token_get = match atcoder_api::scrape_affiliation(&req.atcoder_id).await {
        Ok(affiliation_token) => affiliation_token,
        Err(reason) => {
            return actix_web::HttpResponse::Ok().json(Res {
                status: "error",
                reason: Some(reason.to_string()),
                token: None,
            })
        }
    };
    
    if req.affiliation_token == affiliation_token_get {
        let time = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap().as_secs();
        actix_web::HttpResponse::Ok().json(Res {
            status: "success",
            reason: None,
            token: Some(atcoder_api::create_token(time, &req.atcoder_id)),
        })
    } else {
        actix_web::HttpResponse::Ok().json(Res {
            status: "error",
            reason: Some("affiliation_token not matched".into()),
            token: None,
        })
    }
}