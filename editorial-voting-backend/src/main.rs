pub mod api;
pub mod atcoder_api;

use std::{fs::File, io::BufReader, sync::Mutex};

use actix_web::HttpResponse;

#[actix_web::get("/")]
async fn index() -> HttpResponse {
    HttpResponse::Ok().body("Hello, World!")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // TLS の設定
    let tls_config = {
        let mut tls_certs_file = BufReader::new(File::open(std::env::var("EDITORIAL_VOTING_TLS_CERT_PATH").unwrap())?);
        let mut tls_key_file = BufReader::new(File::open(std::env::var("EDITORIAL_VOTING_TLS_KEY_PATH").unwrap())?);

        let tls_certs = rustls_pemfile::certs(&mut tls_certs_file).collect::<Result<Vec<_>, _>>()?;
        let tls_key = rustls_pemfile::pkcs8_private_keys(&mut tls_key_file).next().unwrap()?;

        rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs8(tls_key))
            .unwrap()
    };

    actix_web::HttpServer::new(move || {
        let cors = actix_cors::Cors::default().allow_any_origin().allow_any_method();
        actix_web::App::new()
            .wrap(cors)
            .app_data(actix_web::web::Data::new(Mutex::new(rusqlite::Connection::open(std::env::var("EDITORIAL_VOTING_DATABASE_PATH").unwrap()).unwrap()) ))
            .service(api::create_affiliation_token::route)
            .service(api::create_token::route)
            .service(api::vote::route)
            .service(api::status::route)
            .service(index)
    })
        .bind_rustls_0_22(std::env::var("EDITORIAL_VOTING_BIND").unwrap(), tls_config)?
        .run()
        .await
}