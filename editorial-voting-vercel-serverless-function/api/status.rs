use std::collections::HashMap;

use editorial_voting_vercel_serverless_function::{atcoder_api, database};
use serde;
use vercel_runtime::{process_request, process_response, run_service, Body, Error, Request, RequestPayloadExt, Response, ServiceBuilder, StatusCode};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Req {
    token: Option<String>,
    editorial: String,
}

#[derive(serde::Serialize, Default, Debug)]
struct Res {
    status: &'static str,
    reason: Option<String>,
    score: Option<i64>,
    scores_by_rating: Option<HashMap<String, i64>>,
    current_vote: Option<&'static str>,
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
        Ok(res) => res,
        Err(reason) => Res { status: "error", reason: Some(reason.to_string()), .. Default::default() },
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Headers", "*")
        .body(Body::Text(serde_json::to_string(&res)?))?)
}

async fn proc(req: Request) -> Result<Res, Box<dyn std::error::Error>> {
    let Ok(Some(req)) = req.payload::<Req>() else {
        return Err("invalid request".into());
    };

    fn use_db(mut client: postgres::Client, req: Req) -> Result<Res, Box<dyn std::error::Error>> {
        let mut user_token = None;
        if let Some(token) = req.token.as_ref() {
            user_token = Some(atcoder_api::parse_token(token)?);
        }

        // get editorial_id
        let Some(editorial_url) = atcoder_api::canonicalize_editorial_url(&req.editorial) else {
            return Err("invalid editorial URL".into());
        };
        
        let Some(row) = client.query_opt("SELECT id FROM editorials WHERE editorial = $1", &[&editorial_url])? else {
            // 未登録
            return Ok(Res {
                status: "success",
                score: Some(0),
                scores_by_rating: Some(HashMap::new()),
                current_vote: user_token.as_ref().map(|_| "none" ),
                .. Default::default()
            });
        };
        let editorial_id = row.get::<_, i32>(0);
        
        // get score
        let mut score = 0;
        let mut scores_by_rating = HashMap::new();
        let rows = client.query("SELECT rating_level, score FROM vote_temp WHERE editorial_id = $1", &[&editorial_id])?;
        for row in rows {
            let rating_level = row.get::<_, i16>(0) as usize;
            let score_by_rating_level = row.get::<_, i32>(1) as i64;
            score += score_by_rating_level;
            scores_by_rating.insert(format!("{}-{}", rating_level * 100, rating_level * 100 + 99), score_by_rating_level);
        }

        let mut current_vote = None;
        if let Some(user_token) = user_token.as_ref() {
            current_vote = Some(match client.query_opt("SELECT score FROM votes WHERE user_id = $1 AND editorial_id = $2", &[&user_token.user_id, &editorial_id])?.map(|row| row.get::<_, i16>(0) ) {
                Some(1) => "up",
                Some(-1) => "down",
                _ => "none"
            });
        }
    
        Ok(Res {
            status: "success",
            score: Some(score),
            scores_by_rating: Some(scores_by_rating),
            current_vote,
            .. Default::default()
        })
    }

    Ok(database::with_database(use_db, req).await?)
}