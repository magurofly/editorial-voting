pub async fn with_database<P: 'static + Send, T: 'static + Send>(f: fn(postgres::Client, param: P) -> Result<T, Box<dyn std::error::Error>>, param: P) -> Result<T, String> {
    tokio::task::spawn_blocking(move || {
        let database_url = std::env::var("EDITORIAL_VOTING_DATABASE_URL").unwrap();

        let mut builder = openssl::ssl::SslConnector::builder(openssl::ssl::SslMethod::tls()).map_err(|e| e.to_string() )?;
        builder.set_ca_file("/etc/ssl/certs/ca-certificates.crt").map_err(|e| e.to_string() )?;

        let connector = postgres_openssl::MakeTlsConnector::new(builder.build());

        let client = postgres::Client::connect(&database_url, connector).map_err(|e| e.to_string() )?;
        f(client, param).map_err(|e| e.to_string() )
    }).await.map_err(|e| e.to_string() )?
}