use std::{collections::HashMap, sync::Mutex};

use actix_web::{self, HttpResponse};
use serde;

use crate::atcoder_api;

#[derive(serde::Deserialize)]
struct Req {
    token: Option<String>,
    editorial: String,
}

#[derive(Default, serde::Serialize)]
struct Res {
    status: &'static str,
    reason: Option<String>,
    score: Option<i64>,
    scores_by_rating: Option<HashMap<String, i64>>,
    current_vote: Option<&'static str>,
}

async fn inner(req: &Req, conn: &rusqlite::Connection) -> Result<Res, Box<dyn std::error::Error>> {
    let mut user_token = None;
    if let Some(token) = req.token.as_ref() {
        user_token = Some(atcoder_api::parse_token(token)?);
    }

    let Ok(editorial_id) = conn.query_row("SELECT id FROM editorials WHERE editorial = ?1", [req.editorial.clone()], |row| row.get::<_, usize>(0) ) else {
        return Ok(Res {
            status: "success",
            reason: None,
            score: Some(0),
            scores_by_rating: Some(HashMap::new()),
            current_vote: user_token.map(|_| "none" ),
        });
    };

    let mut score = 0;
    let mut scores_by_rating = HashMap::new();
    let mut current_vote = None;
    let mut statement = conn.prepare("SELECT rating_level, score FROM vote_temp WHERE editorial_id = ?1")?;
    let mut rows = statement.query([editorial_id])?;
    while let Ok(Some(row)) = rows.next() {
        let level = row.get::<_, i64>(0)?;
        let score_by_level = row.get::<_, i64>(1)?;
        score += score_by_level;
        scores_by_rating.insert(format!("{}-{}", level * 100, level * 100 + 99), score_by_level);
    }

    if let Some(user_token) = user_token {
        let score = conn.query_row("SELECT score FROM votes WHERE user_id = ?1 AND editorial_id = ?2", [user_token.user_id, editorial_id], |row| row.get::<_, i64>(0) ).ok();
        if let Some(score) = score {
            current_vote = Some(match score {
                1 => "up",
                -1 => "down",
                _ => "none"
            });
        } else {
            current_vote = Some("none");
        }
    }

    Ok(Res {
        status: "success",
        reason: None,
        score: Some(score),
        scores_by_rating: Some(scores_by_rating),
        current_vote,
    })
}

#[actix_web::post("/status")]
pub async fn route(req: actix_web::web::Json<Req>, data: actix_web::web::Data<Mutex<rusqlite::Connection>>) -> actix_web::HttpResponse {
    let conn = data.lock().unwrap();
    match inner(&req, &conn).await {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(reason) => HttpResponse::Ok().json(Res {
            status: "error",
            reason: Some(reason.to_string()),
            score: None,
            scores_by_rating: None,
            current_vote: None,
        }),
    }
}
