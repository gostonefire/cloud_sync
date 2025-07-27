use lettre::{AsyncTransport, Message, Tokio1Executor};
use lettre::message::header::ContentType;
use lettre::message::Mailbox;
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::transport::smtp::authentication::Credentials;
use log::error;
use tokio::sync::mpsc::UnboundedReceiver;
use crate::initialization::MailParameters;
use crate::errors::MailError;


/// Sends a mail whenever an event is received over the mpsc channel
/// 
/// # Arguments
/// 
/// * 'config' - mail configuration parameters
/// * 'rx' - mpsc receiver
pub async fn mailer(config: &MailParameters, mut rx: UnboundedReceiver<String>) {
    let sender = match sender(config) {
        Ok(sender) => sender,
        Err(e) => { error!("{}", e); panic!("invalid mail config!") }
    };

    let from = config.from.parse::<Mailbox>().expect("invalid from mailbox config!");
    let to = config.to.parse::<Mailbox>().expect("invalid to mailbox config!");

    loop {
        match rx.recv().await {
            Some(body) => {
                match message(&from, &to, "CloudSync event", body) {
                    Ok(email) => {
                        if let Err(e) = sender.send(email).await {
                            error!("error sending mail: {}", e);
                        }
                    },
                    Err(e) => { error!("{}", e); }
                };
            }
            None => {
                error!("communication channel to mailer terminated");
                break;
            }
        }
    }
}

/// Creates and returns a mail sender
///
/// # Arguments
///
/// * 'config' - mail configuration parameters
fn sender(config: &MailParameters) -> Result<AsyncSmtpTransport<Tokio1Executor>, MailError> {
    let credentials = Credentials::new(config.smtp_user.to_owned(), config.smtp_password.to_owned());
    let sender: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.smtp_endpoint)?
        .credentials(credentials)
        .build();

    Ok(sender)
}

/// Creates a new email message
///
/// # Arguments
///
/// * 'from' - from mail address
/// * 'to' - to mail address
/// * 'subject' - mail subject
/// * 'body' - mail body
fn message(from: &Mailbox, to: &Mailbox, subject: &str, body: String) -> Result<Message, MailError> {
    Ok(Message::builder()
        .from(from.clone())
        .to(to.clone())
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body)?)
}