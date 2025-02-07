pub async fn get_or_insert_user(atcoder_id: &str) -> Result<u32, Box<dyn std::error::Error>> {
    let database_url = std::env::var("EDITORIAL_VOTING_DATABASE_URL")?;
    let mut client = postgres::Client::connect(&database_url, postgres::NoTls)?;

    client.execute("INSERT INTO users(atcoder_id) VALUES($1) ON CONFLICT DO NOTHING;", &[&atcoder_id])?;

    let rows = client.query("SELECT id FROM users WHERE atcoder_id = $1;", &[&atcoder_id])?;
    let Some(row) = rows.get(0) else { return Err("user not found".into()) };

    Ok(row.get::<_, u32>(0))
}

pub async fn list_users() -> Result<String, Box<dyn std::error::Error>> {
    let database_url = std::env::var("EDITORIAL_VOTING_DATABASE_URL")?;
    let (client, connection) = tokio_postgres::connect(&database_url, tokio_postgres::NoTls).await?;
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {e}");
        }
    });

    let rows = client.query("SELECT * FROM users;", &[]).await?;

    Ok(rows.into_iter().map(|row| {
        let user_id = row.get::<_, u32>(0);
        let atcoder_id = row.get::<_, String>(1);
        format!("{user_id}:{atcoder_id}")
    }).collect::<Vec<_>>().join("\n"))
}