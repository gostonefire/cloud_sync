mod initialization;
mod errors;
mod token_manager;
mod onedrive_manager;
mod cloud_sync;
mod onedrive_model;
mod aws_manager;
mod chunk;
mod mail_manager;
mod mail_model;
mod logging;

use log::info;
use std::sync::Arc;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use rustls::{ServerConfig, pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject},};
use reqwest::Url;
use serde::Deserialize;
use tokio::sync::mpsc;
use crate::initialization::{config, Config, OneDrive, WebServerParameters};
use crate::errors::UnrecoverableError;
use crate::cloud_sync::sync;
use crate::mail_manager::mailer;
use crate::token_manager::Tokens;

#[derive(Deserialize)]
struct Params {
    code: String,
}

struct AppState {
    config: Arc<Config>,
}

#[get("/code")]
async fn code(data: web::Data<AppState>, params: web::Query<Params>) -> impl Responder {
    if let Err(e) = Tokens::from_code(&data.config.onedrive, &params.code).await {
        HttpResponse::InternalServerError().body(e.to_string())
    } else {
        HttpResponse::Ok().body("Access granted!")
    }
}

#[actix_web::main]
async fn main() -> Result<(), UnrecoverableError> {
    // Load configuration
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    let config = Arc::new(config(tx)?);
     
    // Mailer
    info!("starting mailer");
    let c = config.clone();
    tokio::spawn(async move { mailer(&c.mail, rx).await });
    
    // Main sync function
    info!("starting main sync function");
    let c = config.clone();
    tokio::spawn(async move { sync(&c).await });

    // Authentication/authorization function
    info!("starting authentication/authorization function");
    let rustls_config = load_rustls_config(&config.web_server)?;
    let c = config.clone();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                config: c.clone(),
            }))
            .service(code)
            .service(web::redirect("/grant", build_access_request_url(&c.clone().onedrive)))
    })
        .workers(4)
        .bind_rustls_0_23((config.web_server.bind_address.as_str(), config.web_server.bind_port), rustls_config)?
        //.bind((config.web_server.bind_address.as_str(), config.web_server.bind_port))?
        .run()
        .await?;
   
    Ok(())
}

/// Builds an access request url and returns a url encoded version of it
///
fn build_access_request_url(config: &OneDrive) -> String {
    let base_url = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
    let params: [(&str, &str); 5] = [
        ("client_id", &config.client_id),
        ("response_type", "code"),
        ("redirect_uri", &config.redirect_uri),
        ("response_mode", "query"),
        ("scope", &config.scope),
    ];

    let url = Url::parse_with_params(base_url, &params).unwrap();
    url.to_string()
}

/// Loads TLS certificates
/// 
/// # Arguments
/// 
/// * 'config' - web server parameters
fn load_rustls_config(config: &WebServerParameters) -> Result<ServerConfig, UnrecoverableError> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    // load TLS key/cert files
    let cert_chain = CertificateDer::pem_file_iter(&config.tls_chain_cert)?
        .flatten()
        .collect();

    let key_der =
        PrivateKeyDer::from_pem_file(&config.tls_private_key).expect("Could not locate PKCS 8 private keys.");

    Ok(ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key_der)?)
}
