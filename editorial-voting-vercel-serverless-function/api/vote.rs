use std::time::{Duration, SystemTime};

use editorial_voting_vercel_serverless_function::{atcoder_api, database};
use serde;
use vercel_runtime::{process_request, process_response, run_service, Body, Error, Request, RequestPayloadExt, Response, ServiceBuilder, StatusCode};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Req {
    token: String,
    contest: String,
    editorial: String,
    vote: String,
}

#[derive(serde::Serialize, Default, Debug)]
struct Res {
    status: &'static str,
    reason: Option<String>,
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
    let res = match proc(req).await {
        Ok(res) => res,
        Err(reason) => Res { status: "error", reason: Some(reason.to_string()), .. Default::default() },
    };
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::Text(serde_json::to_string(&res)?))?)
}

async fn proc(req: Request) -> Result<Res, Box<dyn std::error::Error>> {
    let Ok(Some(req)) = req.payload::<Req>() else {
        return Err("invalid request".into());
    };

    fn use_db(mut client: postgres::Client, req: Req) -> Result<Res, Box<dyn std::error::Error>> {
        // get token
        let user_token = atcoder_api::parse_token(&req.token)?;

        // get editorial_id
        let editorial_id = {
            let Some(editorial_url) = atcoder_api::canonicalize_editorial_url(&req.editorial) else {
                return Err("invalid editorial URL".into());
            };

            if let Ok(row) = client.query_one("SELECT id FROM editorials WHERE editorial = $1", &[&editorial_url]) {
                // already registered
                row.get::<_, i32>(0)
            } else {
                // register all editorials from same contest
                let contest = req.contest.clone();
                let editorials = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?
                    .block_on(async move { atcoder_api::scrape_editorials(&contest).await })?;
                let statement = client.prepare("INSERT INTO editorials(editorial) VALUES($1) ON CONFLICT DO NOTHING")?;
                for editorial in editorials {
                    client.execute(&statement, &[&editorial])?;
                }

                client.query_one("SELECT id FROM editorials WHERE editorial = $1", &[&editorial_url])?.get::<_, i32>(0)
            }
        };
        
        // get old vote and rating
        let (old_vote, old_rating) =
            if let Ok(row) = client.query_one("SELECT score, rating FROM votes WHERE user_id = $1 AND editorial_id = $2", &[&user_token.user_id, &editorial_id]) {
                (row.get::<_, i16>(0), row.get::<_, i16>(1))
            } else {
                (0, 0)
            };

        // get new vote
        let new_vote = match req.vote.as_str() {
            "none" => 0i16,
            "up" => 1i16,
            "down" => -1i16,
            _ => return Err("invalid vote format (none|up|down)".into())
        };
        
        // get new rating
        let mut new_rating = 0i16;
        if new_vote != 0 {
            let mut rating = None;
            // 過去にレーティングを取得したのが 1 時間以内ならそれを使う
            if let Ok(row) = client.query_one("SELECT rating, rating_last_update FROM users WHERE id = $1", &[&user_token.user_id]) {
                let current_rating = row.get::<_, Option<i16>>(0);
                let current_time = row.get::<_, Option<SystemTime>>(1);
                if let Some((current_rating, current_time)) = current_rating.zip(current_time) {
                    if SystemTime::now().duration_since(current_time)? <= Duration::from_secs(60 * 60) {
                        rating = Some(current_rating);
                    }
                }
            }
            // 1 時間よりも古いなら新しく取得
            if rating.is_none() {
                let details = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?
                    .block_on(async move { atcoder_api::scrape_user(&user_token.atcoder_id).await })?;
                let now_time = SystemTime::now();
                rating = Some(details.rating);
                // 保存
                client.execute("UPDATE users SET rating = $1, rating_last_update = $2 WHERE id = $3", &[&details.rating, &now_time, &user_token.user_id])?;
            }
            new_rating = rating.unwrap();
        }
        
        // transaction
        {
            let mut tx = client.transaction()?;

            // remove old vote
            if old_vote != 0 {
                let rating_level = old_rating / 100;
                tx.execute("DELETE FROM votes WHERE user_id = $1 AND editorial_id = $2", &[&user_token.user_id, &editorial_id])?;
                tx.execute("UPDATE vote_temp SET score = score + $1 WHERE editorial_id = $2 AND rating_level = $3", &[&-old_vote, &editorial_id, &rating_level])?;
            }

            // add if new vote is nonzero
            if new_vote != 0 {
                tx.execute("INSERT INTO votes(user_id, editorial_id, score, rating) VALUES($1, $2, $3, $4)", &[&user_token.user_id, &editorial_id, &new_vote, &new_rating])?;
                let rating_level = new_rating / 100;
                tx.execute("INSERT INTO vote_temp(editorial_id, rating_level, score) VALUES($1, $2, 0) ON CONFLICT DO NOTHING", &[&editorial_id, &rating_level])?;
                tx.execute("UPDATE vote_temp SET score = score + $1 WHERE editorial_id = $2 AND rating_level = $3", &[&new_vote, &editorial_id, &rating_level])?;
            }

            tx.commit()?;
        }
    
        Ok(Res {
            status: "success",
            .. Default::default()
        })
    }

    Ok(database::with_database(use_db, req).await?)
}