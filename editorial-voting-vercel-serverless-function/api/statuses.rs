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

#[derive(serde::Serialize, Clone, Default, Debug)]
struct SingleRes {
    score: i64,
    scores_by_rating: HashMap<String, i64>,
    current_vote: Option<&'static str>,
}

#[derive(serde::Serialize, Default, Debug)]
struct EditorialUrlQuery {
    index: i32,
    query: String,
}

#[derive(serde::Serialize, Default, Debug)]
struct IdQuery {
    id: i32,
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
            .header("Access-Control-Allow-Origin", "atcoder.jp")
            .header("Access-Control-Allow-Credentials", "true")
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
        .header("Access-Control-Allow-Origin", "atcoder.jp")
        .header("Access-Control-Allow-Credentials", "true")
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

            // get editorial_ids
            let (editorial_ids, editorial_id_map) = {
                let query_records = req.editorials.iter().enumerate().map(|(index, query)| EditorialUrlQuery { index: index as i32, query: query.to_string() } ).collect::<Vec<_>>();
                let json = serde_json::to_value(&query_records)?;
                let rows = client.query("SELECT id, index FROM editorials, JSON_TO_RECORDSET($1) AS queries(index INTEGER, query TEXT) WHERE editorial = query", &[&json])?;
                // return (id, index)
                let mut editorial_ids = vec![];
                let mut editorial_id_map = HashMap::new();
                for row in rows {
                    let id = row.get::<_, i32>(0);
                    let index = row.get::<_, i32>(1) as usize;
                    editorial_ids.push(id);
                    editorial_id_map.insert(id, index);
                }
                (editorial_ids, editorial_id_map)
            };

            let mut results = vec![SingleRes {
                current_vote: Some("none"),
                ..Default::default()
            }; req.editorials.len()];

            let editorial_ids_json = serde_json::to_value(&editorial_ids.iter().map(|&id| IdQuery { id } ).collect::<Vec<_>>())?;

            // get scores
            {
                let rows = client.query("SELECT id, rating_level, score FROM vote_temp, JSON_TO_RECORDSET($1) AS queries(id INTEGER) WHERE editorial_id = queries.id", &[&editorial_ids_json])?;
                for row in rows {
                    let id = row.get::<_, i32>(0);
                    let rating_level = row.get::<_, i16>(1) as usize;
                    let score_by_rating_level = row.get::<_, i32>(2) as i64;
                    let index = editorial_id_map[&id];
                    results[index].score += score_by_rating_level;
                    results[index].scores_by_rating.insert(format!("{}-{}", rating_level * 100, rating_level * 100 + 99), score_by_rating_level);
                }
            }

            // get current votes
            {
                let rows = client.query("SELECT id, score FROM votes, JSON_TO_RECORDSET($1) AS queries(id INTEGER) WHERE user_id = $2 AND editorial_id = queries.id", &[&editorial_ids_json, &user_token.user_id])?;
                for row in rows {
                    let id = row.get::<_, i32>(0);
                    let score = row.get::<_, i16>(1);
                    let index = editorial_id_map[&id];
                    match score {
                        1 => {
                            results[index].current_vote = Some("up");
                        }
                        -1 => {
                            results[index].current_vote = Some("down");
                        }
                        _ => {}
                    }
                }
            }

            Ok(Res {
                status: "success",
                results: Some(results),
                .. Default::default()
            })
        } else {
            let mut results = vec![SingleRes::default(); req.editorials.len()];

            // get scores
            let query_records = req.editorials.iter().enumerate().map(|(index, query)| EditorialUrlQuery { index: index as i32, query: query.to_string() } ).collect::<Vec<_>>();
            let json = serde_json::to_value(&query_records)?;
            let rows = client.query("SELECT index, rating_level, score FROM editorials, vote_temp, JSON_TO_RECORDSET($1) AS queries(index INTEGER, query TEXT) WHERE editorial_id = id AND editorial = query", &[&json])?;
            for row in rows {
                let index = row.get::<_, i32>(0) as usize;
                let rating_level = row.get::<_, i16>(1) as usize;
                let score_by_rating_level = row.get::<_, i32>(2) as i64;
                results[index].score += score_by_rating_level;
                results[index].scores_by_rating.insert(format!("{}-{}", rating_level * 100, rating_level * 100 + 99), score_by_rating_level);
            }

            Ok(Res {
                status: "success",
                results: Some(results),
                .. Default::default()
            })
        }
    }

    Ok(database::with_database(use_db, req).await?)
}