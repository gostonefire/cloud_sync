mod config;
mod errors;
mod token_manager;
mod onedrive_manager;

use std::sync::Arc;
use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use reqwest::Url;
use serde::Deserialize;
use crate::config::{config, Config, OneDrive};
use crate::errors::UnrecoverableError;
use crate::onedrive_manager::list_drives;
use crate::token_manager::Tokens;

#[derive(Deserialize)]
struct Params {
    code: String,
}

struct AppState {
    config: Arc<Config>,
}

#[get("/code")]
async fn hello(data: web::Data<AppState>, params: web::Query<Params>) -> impl Responder {
    if let Err(e) = Tokens::from_code(&data.config.onedrive, &params.code).await {
        HttpResponse::InternalServerError().body(e.to_string())
    } else {
        HttpResponse::Ok().body("Access granted!")
    }
}

#[actix_web::main]
async fn main() -> Result<(), UnrecoverableError> {
    // Load configuration
    let config = Arc::new(config()?);
     
    // Main sync function
    let c = config.clone();
    tokio::spawn(async move { list_drives(&c.onedrive).await });

    // Authentication/authorization function
    let c = config.clone();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                config: c.clone(),
            }))
            .service(hello)
            .service(web::redirect("/grant", build_access_request_url(&c.clone().onedrive)))
    })
        .workers(4)
        .bind(("127.0.0.1", 8000))?
        .run()
        .await?;
   
    Ok(())
}

/// Builds an access request url and returns an url encoded version of it
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

