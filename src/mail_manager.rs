use std::time::Duration;
use log::{error, warn};
use tokio::sync::mpsc::UnboundedReceiver;
use crate::initialization::MailParameters;
use crate::errors::MailError;
use crate::mail_model::{Address, Content, Email, Personalizations};

pub struct Mail {
    api_key: String,
    client: reqwest::Client,
    from: Address,
    to: Address,
}

impl Mail {
    /// Returns a new instance of the Mail struct
    ///
    /// # Arguments
    ///
    /// * 'api_key' - the api key for sendgrid
    /// * 'from' - sender email address
    /// * 'to' - receiver email address
    pub fn new(config: &MailParameters) -> Result<Self, MailError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(
            Self {
                client,
                api_key: config.api_key.to_string(),
                from: config.from.parse::<Address>()?,
                to: config.to.parse::<Address>()?,
            }
        )
    }

    /// Sends a mail with the given subject and body
    ///
    /// # Arguments
    ///
    /// * 'subject' - the subject of the mail
    /// * 'body' - the body of the mail
    pub async fn send_mail(&self, subject: String, body: String) -> Result<(), MailError> {

        // Temporary inhibiting sending mail until new mailer is configured
        // Sendgrid will stop free email sending accounts on the 26/7 2025
        Ok(())

        /*
        let req = Email {
            personalizations: vec![Personalizations { to: vec![self.to.clone()]}],
            from: self.from.clone(),
            subject,
            content: vec![Content { content_type: "text/plain".to_string(), value: body }],
        };

        let json = serde_json::to_string(&req)?;

        let _ = self.client
            .post("https://api.sendgrid.com/v3/mail/send")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .body(json)
            .send()
            .await?;

        Ok(())

         */
    }
}

/// Sends a mail whenever an event is received over the mpsc channel
/// 
/// # Arguments
/// 
/// * 'config' - mail configuration parameters
/// * 'rx' - mpsc receiver
pub async fn mailer(config: &MailParameters, mut rx: UnboundedReceiver<String>) {
    let mail = if let Ok(mail) = Mail::new(config) {
        mail
    } else {
        error!("unable to create mail client");
        return;
    };
    
    loop {
        match rx.recv().await {
            Some(body) => {
                match mail.send_mail("CloudSync event".to_string(), body).await {
                    Err(err) => {
                        error!("error sending mail: {}", err);
                    }
                    Ok(_) => (),
                }
            }
            None => { 
                warn!("communication channel to mailer terminated");
                break;
            }
        }
    }
}