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
    let handler = ServiceBuilder::new()
        .map_request(process_request)
        .map_response(process_response)
        .service_fn(handler);

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
        Ok(token) => Res { status: "success", token: Some(token), .. Default::default() },
        Err(reason) => Res { status: "error", reason: Some(reason.to_string()), .. Default::default() },
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "*")
        .body(Body::Text(serde_json::to_string(&res)?))?)
}

async fn proc(req: Request) -> Result<String, Box<dyn std::error::Error>> {
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

    // connect database
    fn use_db(mut client: postgres::Client, atcoder_id: String) -> Result<i32, Box<dyn std::error::Error>> {
        client.execute("INSERT INTO users(atcoder_id) VALUES($1) ON CONFLICT DO NOTHING", &[&atcoder_id])?;

        let row = client.query_one("SELECT id FROM users WHERE atcoder_id = $1", &[&atcoder_id])?;

        Ok(row.get::<_, i32>(0))
    }
    let user_id = database::with_database(use_db, req.atcoder_id.clone()).await?;

    let token = atcoder_api::create_token(time, &req.atcoder_id, user_id)?;

    Ok(token)
}