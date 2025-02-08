use std::sync::{Arc, Mutex};

use editorial_voting_vercel_serverless_function::{atcoder_api, database};
use serde;
use vercel_runtime::{process_request, process_response, run_service, Body, Error, Request, RequestPayloadExt, Response, ServiceBuilder, StatusCode};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Req {
    atcoder_id: String,
    affiliation_token: String,
}

#[derive(serde::Serialize, Default, Debug)]
struct Res {
    status: &'static str,
    reason: Option<String>,
    token: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let middleware = database::DatabaseMiddleware::new();

    let handler = ServiceBuilder::new()
        .map_request(process_request)
        .map_response(process_response)
        .service(middleware.service_fn(handler));

    run_service(handler).await
}

pub async fn handler((req, client_mutex): (Request, Arc<Mutex<Option<tokio_postgres::Client>>>)) -> Result<Response<Body>, Error> {
    let res = match proc(req, client_mutex).await {
        Ok(token) => Res { status: "success", token: Some(token), .. Default::default() },
        Err(reason) => Res { status: "error", reason: Some(reason.to_string()), .. Default::default() },
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::Text(serde_json::to_string(&res)?))?)
}

pub async fn proc(req: Request, client_mutex: Arc<Mutex<Option<tokio_postgres::Client>>>) -> Result<String, Box<dyn std::error::Error>> {
    let time = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH)?.as_secs();

    let Ok(Some(req)) = req.payload::<Req>() else {
        return Err("invalid request".into());
    };
    
    if !atcoder_api::validate_atcoder_id(&req.atcoder_id) {
        return Err("invalid atcoder_id format".into());
    }
    atcoder_api::validate_affiliation_token(&req.atcoder_id, &req.affiliation_token)?;

    // fetch affiliation_token from AtCoder user page
    let affiliation_token = atcoder_api::scrape_affiliation(&req.atcoder_id).await?;

    if affiliation_token != req.affiliation_token {
        return Err("affiliation token not matched".into());
    }

    let Ok(mut client_opt) = client_mutex.lock() else { return Err("lock error".into()) };
    let Some(client) = client_opt.as_mut() else { return Err("database client not found".into()) };

    client.execute("INSERT INTO users(atcoder_id) VALUES($1) ON CONFLICT DO NOTHING;", &[&req.atcoder_id]).await?;

    let rows = client.query("SELECT id FROM users WHERE atcoder_id = $1;", &[&req.atcoder_id]).await?;
    let Some(row) = rows.get(0) else { return Err("user not found".into()) };

    let user_id = row.get::<_, i32>(0) as u32;

    let token = atcoder_api::create_token(time, &req.atcoder_id, user_id)?;

    Ok(token)
}