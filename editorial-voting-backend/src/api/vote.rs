use std::sync::Mutex;

use actix_web::{self, HttpResponse};
use serde;

use crate::atcoder_api::{self, AtCoderUserDetails};

#[derive(serde::Deserialize)]
struct Req {
    token: String,
    contest: String,
    editorial: String,
    vote: String,
}

#[derive(serde::Serialize)]
struct Res {
    status: &'static str,
    reason: Option<String>,
}

async fn inner(req: &Req, db: &Mutex<rusqlite::Connection>) -> Result<Res, Box<dyn std::error::Error>> {
    // check if token is valid and parse token
    let user_token = atcoder_api::parse_token(&req.token)?;

    // check if vote is valid and parse
    let new_vote = match req.vote.as_str() {
        "none" => 0,
        "up" => 1,
        "down" => -1,
        _ => Err("invalid vote format".to_string())?
    };

    // check if editorial is already registered
    let editorial_id = {
        if let Ok(editorial_id) = db.lock().unwrap().query_row("SELECT id FROM editorials WHERE editorial = ?1", [&req.editorial], |row| row.get::<_, usize>(0) ) {
            editorial_id
        } else {
            // register all editorials from same contest
            let scraped_editorials = atcoder_api::scrape_editorials(&req.contest).await?;

            let conn = db.lock().unwrap();
            for editorial in scraped_editorials {
                conn.execute("INSERT OR IGNORE INTO editorials(editorial) VALUES(?1)", [editorial])?;
            }

            // re-check and if none, the editorial does not exist
            conn.query_row("SELECT id FROM editorials WHERE editorial = ?1", [&req.editorial], |row| row.get::<_, usize>(0) ).map_err(|_| "given editorial does not exist".to_string() )?
        }
    };

    // get old vote and rating
    let (old_vote, old_rating) =
        if let Ok(vote_rating) = db.lock().unwrap().query_row("SELECT score, rating FROM votes WHERE user_id = ?1 AND editorial_id = ?2", [user_token.user_id, editorial_id], |row| (0 .. 2).map(|i| row.get::<_, i64>(i) ).collect::<Result<Vec<_>, _>>() ) {
            (vote_rating[0], vote_rating[1])
        } else {
            (0, 0)
        };
    
    // get new rating
    let new_rating =
        if new_vote != 0 {
            // 投票する場合レートを取得
            if let Ok(res) = db.lock().unwrap().query_row("SELECT rating, rating_last_update FROM users WHERE id = ?1", [user_token.user_id], |row| (0 .. 2).map(|i|  row.get::<_, i64>(i) ).collect::<Result<Vec<_>, _>>() ) {
                let rating = res[0];
                let rating_last_update = std::time::SystemTime::UNIX_EPOCH.checked_add(std::time::Duration::from_secs(res[1] as u64)).unwrap();
                let now = std::time::SystemTime::now();
                // update rating if last update is over 1 hour ago
                if now.duration_since(rating_last_update)? > std::time::Duration::from_secs(60 * 60) {
                    let AtCoderUserDetails { rating } = atcoder_api::scrape_user(&user_token.atcoder_id).await?;
                    let now_time = now.duration_since(std::time::SystemTime::UNIX_EPOCH)?.as_secs() as i64;
                    db.lock().unwrap().execute("UPDATE users SET rating = ?1, rating_last_update = ?2 WHERE id = ?3", [rating, now_time, user_token.user_id as i64])?;
                    rating
                } else {
                    rating
                }
            } else {
                // update rating if null
                let AtCoderUserDetails { rating } = atcoder_api::scrape_user(&user_token.atcoder_id).await?;
                let now_time = std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH)?.as_secs() as i64;
                db.lock().unwrap().execute("UPDATE users SET rating = ?1, rating_last_update = ?2 WHERE id = ?3", [rating, now_time, user_token.user_id as i64])?;
                rating
            }
        } else {
            // 投票しないのでレートが不要
            0
        };

    {
        let mut conn = db.lock().unwrap();
        // begin transaction
        let tx = conn.transaction()?;

        // remove old vote
        if old_vote != 0 {
            let rating_level = old_rating / 100;
            tx.execute("DELETE FROM votes WHERE user_id = ?1 AND editorial_id = ?2", [user_token.user_id, editorial_id])?;
            tx.execute("UPDATE vote_temp SET score = score + ?1 WHERE editorial_id = ?2 AND rating_level = ?3", [-old_vote, editorial_id as i64, rating_level])?;
        }

        // add if new vote is not zero
        if new_vote != 0 {
            // update table votes
            tx.execute("INSERT INTO votes(user_id, editorial_id, score, rating) VALUES(?1, ?2, ?3, ?4)", [user_token.user_id as i64, editorial_id as i64, new_vote, new_rating])?;

            // update table vote_temp
            let rating_level = new_rating / 100;
            tx.execute("INSERT OR IGNORE INTO vote_temp(editorial_id, rating_level, score) VALUES(?1, ?2, 0)", [editorial_id as i64, rating_level])?;
            tx.execute("UPDATE vote_temp SET score = score + ?1 WHERE editorial_id = ?2 AND rating_level = ?3", [new_vote, editorial_id as i64, rating_level])?;
        }

        // end transaction
        tx.commit()?;
    }

    Ok(Res {
        status: "success",
        reason: None,
    })
}

#[actix_web::post("/vote")]
pub async fn route(req: actix_web::web::Json<Req>, data: actix_web::web::Data<Mutex<rusqlite::Connection>>) -> actix_web::HttpResponse {
    match inner(&req, &data).await {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(reason) => HttpResponse::Ok().json(Res {
            status: "error",
            reason: Some(reason.to_string()),
        }),
    }
}
