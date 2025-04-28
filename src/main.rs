mod config;
mod errors;
mod token_manager;
mod onedrive_manager;

use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use reqwest::Url;
use serde::Deserialize;
use crate::config::{config, OneDrive};
use crate::errors::UnrecoverableError;
use crate::onedrive_manager::list_drives;
use crate::token_manager::Tokens;

#[derive(Deserialize)]
struct Params {
    code: String,
}

struct AppState {
    onedrive: OneDrive,
}

#[get("/code")]
async fn hello(data: web::Data<AppState>, params: web::Query<Params>) -> impl Responder {
    if let Err(e) = Tokens::from_code(&data.onedrive, &params.code).await {
        HttpResponse::InternalServerError().body(e.to_string())
    } else {
        HttpResponse::Ok().body("Access granted!")
    }
}

#[actix_web::main]
async fn main() -> Result<(), UnrecoverableError> {
    // Load configuration
    let config = config()?;
    
    // Main sync function
    list_drives(&config.onedrive).await;

    // Authentication/authorization function
    let config_onedrive = config.onedrive.clone();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                onedrive: config_onedrive.clone(),
            }))
            .service(hello)
            .service(web::redirect("/grant", build_access_request_url()))
    })
        .workers(4)
        .bind(("127.0.0.1", 8000))?
        .run()
        .await?;
    
    Ok(())
}

/// Builds an access request url and returns an url encoded version of it
///
fn build_access_request_url() -> String {
    let base_url = "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize";
    let params = [
        ("client_id", "067c54ae-88b9-41e9-9e73-ad348da01fc4"),
        ("response_type", "code"),
        ("redirect_uri", "http://localhost:8000/code"),
        ("response_mode", "query"),
        ("scope", "offline_access Files.Read Files.Read.All Files.ReadWrite Files.ReadWrite.All"),
    ];

    let url = Url::parse_with_params(base_url, &params).unwrap();
    url.to_string()
}

