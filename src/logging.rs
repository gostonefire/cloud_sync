use std::io::Write;
use std::sync::{Arc, Mutex};
use derivative::Derivative;
use log4rs::append::Append;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Logger, Root};
use log4rs::encode::Encode;
use log4rs::encode::pattern::PatternEncoder;
use log::{LevelFilter, Record};
use log4rs::encode::writer::simple::SimpleWriter;
use tokio::sync::mpsc::UnboundedSender;
use crate::errors::ConfigError;

/// Sets up the logger
///
/// # Arguments
///
/// * 'log_path' - path where to save logs
/// * 'tx' - mpsc sender
pub fn setup_logger(log_path: &str, tx: UnboundedSender<String>) -> Result<(), ConfigError> {
    let mail = MailAppender::builder()
        .encoder(Box::new(PatternEncoder::new("[{d(%Y-%m-%d %H:%M:%S %:z)} {l} {M}] - {m}{n}")))
        .writer(tx)
        .build();

    let file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("[{d(%Y-%m-%dT%H:%M:%S%.f):0<29}{d(%:z)} {l} {M}] - {m}{n}")))
        .build(log_path)?;


    let config = log4rs::Config::builder()
        .appender(Appender::builder().build("file", Box::new(file)))
        .appender(Appender::builder().build("mail", Box::new(mail)))
        .logger(Logger::builder()
            .appender("mail")
            .additive(true)
            .build("mail", LevelFilter::Info))
        .build(Root::builder()
            .appenders(["file"]).build(LevelFilter::Info)

        )?;

    let _ = log4rs::init_config(config)?;

    Ok(())
}

/// A builder for `MailAppender`s
///
struct MailAppenderBuilder {
    encoder: Option<Box<dyn Encode>>,
    writer: Option<UnboundedSender<String>>
}

impl MailAppenderBuilder {
    /// Sets the output encoder for the `MailAppender`
    ///
    /// # Arguments
    /// 
    /// * 'encoder' - encoder to use for encoding the log message
    fn encoder(mut self, encoder: Box<dyn Encode>) -> MailAppenderBuilder {
        self.encoder = Some(encoder);
        self
    }

    /// Sets the output writer for the `MailAppender`
    /// 
    /// Since the `Append` trait isn't an async method we can't use the `Mail` struct directly
    /// but rather using a `tokio::unbounded_channel` to communicate with a spawned mail
    /// client loop which holds the `UnboundedReceiver` in the other end.
    /// 
    /// # Arguments
    /// 
    /// * 'writer' - expects an unbounded channel sender
    fn writer(mut self, writer: UnboundedSender<String>) -> MailAppenderBuilder {
        self.writer = Some(writer);
        self
    }

    /// Consumes the `MailAppenderBuilder`, producing a `MailAppender`
    ///
    fn build(self) -> MailAppender {
        MailAppender {
            encoder: self
                .encoder
                .unwrap_or_else(|| Box::<PatternEncoder>::default()),
            writer: self.writer.unwrap(),
        }
    }
}

/// An appender which logs via mail
///
#[derive(Derivative)]
#[derivative(Debug)]
struct MailAppender {
    encoder: Box<dyn Encode>,
    #[derivative(Debug = "ignore")]
    writer: UnboundedSender<String>,
}
impl MailAppender {
    /// Creates a new `MailAppender` builder
    ///
    fn builder() -> MailAppenderBuilder {
        MailAppenderBuilder {
            encoder: None,
            writer: None,
        }
    }
}
impl Append for MailAppender {
    /// Implementation of the `append` trait which gets a log record, encodes it to a buffer
    /// and is then sent to the spawned mail client loop
    /// 
    fn append(&self, record: &Record) -> anyhow::Result<()> {
        let data: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let buffer = Buffer::new(data.clone());
        self.encoder.encode(&mut SimpleWriter(buffer), record)?;

        let text = String::from_utf8(data.lock().unwrap().to_vec())?;

        self.writer.send(text)?;

        Ok(())
    }

    fn flush(&self) {}
}

/// Buffer to collect log formatted output from log4rs SimpleWriter
/// This is unfortunately needed since the SimpleWriter couldn't accept any references but
/// needed to move whatever writer was given to it. Hence, an Arc<Mutex<Vec<u8>>> was needed 
/// which can be cloned and moved while retaining an outer ownership.
///
struct Buffer {
    buf: Arc<Mutex<Vec<u8>>>,
}
impl Buffer {
    fn new(buf: Arc<Mutex<Vec<u8>>>) -> Buffer {
        Buffer { buf }
    }
}
impl Write for Buffer{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.buf.lock().unwrap().write(buf)

    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
