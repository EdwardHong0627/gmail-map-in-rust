use anyhow::{Context, Result};
use lettre::message::header::ContentType;
use lettre::message::{Attachment, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, AsyncSmtpTransport, Tokio1Executor, AsyncTransport};
use std::path::Path;
use tokio::fs;

/// A client wrapper for sending emails via Gmail SMTP.
pub struct GmailClient {
    username: String,
    password: String,
}

impl GmailClient {
    /// Creates a new GmailClient instance.
    ///
    /// # Arguments
    ///
    /// * `username` - Gmail email address (e.g., "user@gmail.com").
    /// * `password` - Google App Password.
    pub fn new(username: String, password: String) -> Self {
        Self { username, password }
    }

    /// Sends an email using Gmail SMTP.
    ///
    /// # Arguments
    ///
    /// * `to` - Recipient email address.
    /// * `subject` - Email subject.
    /// * `body` - Plain text body of the email.
    /// * `attachment_path` - Optional absolute path to a file to attach.
    pub async fn send_email(&self, to: &str, subject: &str, body: &str, attachment_path: Option<&str>) -> Result<String> {
        let email_builder = Message::builder()
            .from(self.username.parse().context("Invalid 'from' address")?)
            .to(to.parse().context("Invalid 'to' address")?)
            .subject(subject);

        let email_body = if let Some(path) = attachment_path {
            let path_obj = Path::new(path);
            let filename = path_obj.file_name().unwrap_or_default().to_string_lossy().to_string();
            let content = fs::read(path).await.context("Failed to read attachment file")?;
            let mime_type = mime_guess::from_path(path).first_or_octet_stream();
            let content_type = ContentType::parse(mime_type.as_ref()).map_err(|_| anyhow::anyhow!("Invalid content type"))?;

            let attachment = Attachment::new(filename)
                .body(content, content_type);

            MultiPart::mixed()
                .singlepart(SinglePart::plain(body.to_string()))
                .singlepart(attachment)
        } else {
            // lettre requires a MultiPart or SinglePart body structure usually, 
            // but for simple text we can just use SinglePart::plain wrapped or just text.
            // Actually Message::builder() expects a body.
             MultiPart::mixed()
                .singlepart(SinglePart::plain(body.to_string()))
        };

        let email = email_builder
            .multipart(email_body)
            .context("Failed to build email")?;

        let creds = Credentials::new(self.username.clone(), self.password.clone());

        // Open a remote connection to gmail
        let mailer: AsyncSmtpTransport<Tokio1Executor> = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.gmail.com")
            .context("Failed to build SMTP transport")?
            .credentials(creds)
            .build();

        // Send the email
        match mailer.send(email).await {
            Ok(response) => {
                // lettre response doesn't always have a message ID easily accessible in string format like API,
                // but usually returns a response struct. We'll return "Sent" or the Debug string.
                Ok(format!("Sent: {:?}", response))
            }
            Err(e) => Err(anyhow::anyhow!("Failed to send email: {}", e)),
        }
    }
}
