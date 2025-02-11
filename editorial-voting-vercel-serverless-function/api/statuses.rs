use std::collections::HashMap;

use editorial_voting_vercel_serverless_function::{atcoder_api, database};
use serde;
use vercel_runtime::{process_request, process_response, run_service, Body, Error, Request, RequestPayloadExt, Response, ServiceBuilder, StatusCode};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Req {
    token: Option<String>,
    editorials: Vec<String>,
}

#[derive(serde::Serialize, Default, Debug)]
struct Res {
    status: &'static str,
    reason: Option<String>,
    results: Option<Vec<SingleRes>>,
}

#[derive(serde::Serialize, Default, Debug)]
struct SingleRes {
    score: i64,
    scores_by_rating: HashMap<String, i64>,
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

    if req.editorials.len() > 256 {
        return Err("number of editorials must be less than or equal to 256".into());
    }

    fn use_db(mut client: postgres::Client, req: Req) -> Result<Res, Box<dyn std::error::Error>> {
        if let Some(token) = req.token.as_ref() {
            let user_token = atcoder_api::parse_token(token)?;

            let results = req.editorials.iter().map(|editorial| -> Result<SingleRes, Box<dyn std::error::Error>> {
                // get editorial_id
                let Some(editorial_url) = atcoder_api::canonicalize_editorial_url(&editorial) else {
                    return Err("invalid editorial URL".into());
                };
                let rows = client.query("SELECT id FROM editorials WHERE editorial = $1", &[&editorial_url])?;
                if rows.is_empty() {
                    return Ok(SingleRes {
                        score: 0,
                        current_vote: Some("none"),
                        .. Default::default()
                    });
                }
                let editorial_id = rows[0].get::<_, i32>(0);

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
                
                // get current vote
                let current_vote = Some(match client.query_opt("SELECT score FROM votes WHERE user_id = $1 AND editorial_id = $2", &[&user_token.user_id, &editorial_id])?.map(|row| row.get::<_, i16>(0) ) {
                    Some(1) => "up",
                    Some(-1) => "down",
                    _ => "none"
                });

                Ok(SingleRes {
                    score,
                    scores_by_rating,
                    current_vote,
                })
            }).collect::<Result<Vec<SingleRes>, _>>()?;

            Ok(Res {
                status: "success",
                results: Some(results),
                .. Default::default()
            })
        } else {
            let results = req.editorials.iter().map(|editorial| -> Result<SingleRes, Box<dyn std::error::Error>> {
                // get score
                let rows = client.query("SELECT rating_level, score FROM vote_temp, editorials WHERE editorial_id = editorials.id AND editorials.editorial = $1", &[editorial])?;
                let mut score = 0;
                let mut scores_by_rating = HashMap::new();
                for row in rows {
                    let rating_level = row.get::<_, i16>(0) as usize;
                    let score_by_rating_level = row.get::<_, i32>(1) as i64;
                    score += score_by_rating_level;
                    scores_by_rating.insert(format!("{}-{}", rating_level * 100, rating_level * 100 + 99), score_by_rating_level);
                }

                Ok(SingleRes {
                    score,
                    scores_by_rating,
                    .. Default::default()
                })
            }).collect::<Result<Vec<SingleRes>, _>>()?;

            Ok(Res {
                status: "success",
                results: Some(results),
                .. Default::default()
            })
        }
    }

    Ok(database::with_database(use_db, req).await?)
}