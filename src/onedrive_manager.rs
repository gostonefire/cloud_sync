use crate::config::OneDrive;
use crate::token_manager::Tokens;

pub async fn list_drives(config: &OneDrive) {
    let mut tokens = Tokens::from_file(&config.tokens_path).await.unwrap();
    if tokens.is_expired() {
        tokens.refresh_tokens(config).await.unwrap();
    }

    let access_token = tokens.get_access_token();
    let auth = format!("Bearer {}", access_token);

    let client = reqwest::Client::new();
    let res = client
        .get("https://graph.microsoft.com/v1.0/me/drives")
        .header("Authorization", auth)
        .send()
        .await
        .unwrap();

    let status = res.status();
    let text = res.text().await.unwrap();
    println!("Status: {}\nPayload:\n{}", status, text);
}
