use editorial_voting_vercel_serverless_function::atcoder_api;
use serde;
use vercel_runtime::{process_request, process_response, run_service, service_fn, Body, Error, Request, RequestPayloadExt, Response, ServiceBuilder, StatusCode};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Req {
    atcoder_id: String,
}

#[derive(serde::Serialize, Default, Debug)]
struct Res {
    status: &'static str,
    reason: Option<String>,
    affiliation_token: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let handler = ServiceBuilder::new()
        .map_request(process_request)
        .map_response(process_response)
        .service(service_fn(handler));

    run_service(handler).await
}

pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    if req.method() == "OPTIONS" {
        return Ok(Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "*")
            .header("Access-Control-Allow-Headers", "*")
            .header("Access-Control-Max-Age", "86400")
            .body(Body::Empty)?);
    }
    let res = match proc(req).await {
        Ok(affiliation_token) => Res { status: "success", affiliation_token: Some(affiliation_token), .. Default::default() },
        Err(reason) => Res { status: "error", reason: Some(reason.to_string()), .. Default::default() },
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "*")
        .body(Body::Text(serde_json::to_string(&res)?))?)
}

pub async fn proc(req: Request) -> Result<String, Box<dyn std::error::Error>> {
    let Ok(Some(req)) = req.payload::<Req>() else {
        return Err("invalid request".into());
    };
    
    if !atcoder_api::validate_atcoder_id(&req.atcoder_id) {
        return Err("invalid atcoder_id format".into());
    }

    let time = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH)?.as_secs();
    let affiliation_token = atcoder_api::create_affiliation_token(time, &req.atcoder_id)?;

    Ok(affiliation_token)
}