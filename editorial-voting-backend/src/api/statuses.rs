use std::{collections::HashMap, sync::Mutex};

use actix_web::{self, HttpResponse};
use serde;

use crate::atcoder_api;

#[derive(serde::Deserialize)]
struct Req {
    token: Option<String>,
    editorials: Vec<String>,
}

#[derive(Default, serde::Serialize)]
struct Res {
    status: &'static str,
    reason: Option<String>,
    results: Option<Vec<SingleResult>>,
}

#[derive(Clone, Default, serde::Serialize)]
struct SingleResult {
    score: i64,
    scores_by_rating: HashMap<String, i64>,
    current_vote: Option<&'static str>,
}

async fn inner(req: &Req, conn: &rusqlite::Connection) -> Result<Res, Box<dyn std::error::Error>> {
    if req.editorials.len() > 256 {
        return Err("number of editorials must be less than or equal to 256".into());
    }

    let mut user_token = None;
    if let Some(token) = req.token.as_ref() {
        user_token = Some(atcoder_api::parse_token(token)?);
    }

    let mut get_vote_temp = conn.prepare("SELECT rating_level, score FROM vote_temp WHERE editorial_id = ?1")?;

    let mut results = vec![SingleResult { current_vote: if user_token.is_some() { Some("none") } else { None }, ..SingleResult::default() }; req.editorials.len()];
    for i in 0 .. req.editorials.len() {
        let Ok(editorial_id) = conn.query_row("SELECT id FROM editorials WHERE editorial = ?1", [&req.editorials[i]], |row| row.get::<_, usize>(0) ) else { continue };

        let mut rows = get_vote_temp.query([editorial_id])?;
        while let Ok(Some(row)) = rows.next() {
            let level = row.get::<_, i64>(0)?;
            let score_by_level = row.get::<_, i64>(1)?;
            results[i].score += score_by_level;
            results[i].scores_by_rating.insert(format!("{}-{}", level * 100, level * 100 + 99), score_by_level);
        }

        if let Some(user_token) = &user_token {
            let score = conn.query_row("SELECT score FROM votes WHERE user_id = ?1 AND editorial_id = ?2", [user_token.user_id, editorial_id], |row| row.get::<_, i64>(0) ).ok();
            if let Some(score) = score {
                results[i].current_vote = Some(match score {
                    1 => "up",
                    -1 => "down",
                    _ => "none"
                });
            } else {
                results[i].current_vote = Some("none");
            }
        }
    }

    Ok(Res {
        status: "success",
        reason: None,
        results: Some(results),
    })
}

#[actix_web::post("/statuses")]
pub async fn route(req: actix_web::web::Json<Req>, data: actix_web::web::Data<Mutex<rusqlite::Connection>>) -> actix_web::HttpResponse {
    let conn = data.lock().unwrap();
    match inner(&req, &conn).await {
        Ok(res) => HttpResponse::Ok().json(res),
        Err(reason) => HttpResponse::Ok().json(Res {
            status: "error",
            reason: Some(reason.to_string()),
            results: None,
        }),
    }
}
