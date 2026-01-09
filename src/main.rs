mod initialization;
mod errors;
mod token_manager;
mod onedrive_manager;
mod cloud_sync;
mod onedrive_model;
mod aws_manager;
mod chunk;
mod mail_manager;
mod logging;

use log::{error, info};
use std::sync::Arc;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use axum::extract::{Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Redirect};
use axum::Router;
use axum::routing::get;
use reqwest::Url;
use serde::Deserialize;
use tokio::sync::mpsc;
use crate::initialization::{config, Config, OneDrive};
use crate::errors::UnrecoverableError;
use crate::cloud_sync::sync;
use crate::mail_manager::mailer;
use crate::token_manager::Tokens;

pub type SharedState = Arc<Config>;

#[derive(Deserialize)]
struct Params {
    code: String,
}

async fn code(State(state): State<SharedState>, Query(params): Query<Params>) -> impl IntoResponse {
    if let Err(e) = Tokens::from_code(&state.onedrive, &params.code).await {
        (StatusCode::INTERNAL_SERVER_ERROR, [(header::CONTENT_TYPE, "text/plain")], e.to_string())
            .into_response()

    } else {
        ([(header::CONTENT_TYPE, "text/plain")], "Access granted!")
            .into_response()
    }
}

#[tokio::main]
async fn main() -> Result<(), UnrecoverableError> {
    // Load configuration
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    let config: SharedState = Arc::new(config(tx)?);
     
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

    let redirect_url = build_access_request_url(&config.onedrive);

    let app = Router::new()
        .route("/code", get(code))
        .route("/grant", get(|| async move { Redirect::to(&redirect_url) }))
        .with_state(config.clone());

    let ip_addr = Ipv4Addr::from_str(&config.web_server.bind_address).expect("invalid BIND_ADDR");
    let addr = SocketAddr::new(IpAddr::V4(ip_addr), config.web_server.bind_port);

    let result = axum_server::bind(addr)
        .serve(app.into_make_service())
        .await;

    if let Err(e) = result {
        error!("server error: {}", e);
        Err(UnrecoverableError(format!("server error: {}", e)))?
    } else {
        Ok(())
    }
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
