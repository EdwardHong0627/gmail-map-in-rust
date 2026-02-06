use google_gmail1::Gmail;
use google_gmail1::api::Message;
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_rustls::HttpsConnector;
use yup_oauth2::{read_application_secret, InstalledFlowAuthenticator, InstalledFlowReturnMethod};
use anyhow::{Context, Result};
use base64::prelude::*;
use std::path::Path;
use tokio::fs;
use mime_guess::from_path;

/// A client wrapper for the Gmail API.
pub struct GmailClient {
    // Gmail struct is generic over the client connector
    hub: Gmail<HttpsConnector<HttpConnector>>,
}

impl GmailClient {
    /// Creates a new GmailClient instance.
    ///
    /// This method tries to find credentials in this order:
    /// 1. `GOOGLE_CLIENT_SECRET` environment variable: Must contain the base64-ish or JSON content details.
    /// 2. `client_secret.json` file: Must exist at `secret_path`.
    ///
    /// It uses the `InstalledFlowAuthenticator` to handle the OAuth2 flow. Tokens are persisted to `token_cache.json`.
    pub async fn new(secret_path: &str) -> Result<Self> {
        let secret = if let Ok(secret_json) = std::env::var("GOOGLE_CLIENT_SECRET") {
            yup_oauth2::parse_application_secret(&secret_json)
                .context("Failed to parse GOOGLE_CLIENT_SECRET env var")?
        } else {
            read_application_secret(secret_path)
                .await
                .context("Failed to read client secret file. Please ensure 'client_secret.json' exists or GOOGLE_CLIENT_SECRET env var is set.")?
        };

        let auth = InstalledFlowAuthenticator::builder(
            secret,
            InstalledFlowReturnMethod::HTTPRedirect,
        )
        .persist_tokens_to_disk("token_cache.json")
        .build()
        .await
        .context("Failed to build authenticator")?;

        let client = Client::builder(hyper_util::rt::TokioExecutor::new())
            .build(
                hyper_rustls::HttpsConnectorBuilder::new()
                    .with_native_roots()
                    .expect("no native roots")
                    .https_or_http()
                    .enable_http1()
                    .build(),
            );

        let hub = Gmail::new(client, auth);

        Ok(Self { hub })
    }

    /// Sends an email using the Gmail API.
    ///
    /// Constructs a `multipart/mixed` MIME message to support both body text and optional attachments.
    ///
    /// # Arguments
    ///
    /// * `to` - Recipient email address.
    /// * `subject` - Email subject.
    /// * `body` - Plain text body of the email.
    /// * `attachment_path` - Optional absolute path to a file to attach.
    pub async fn send_email(&self, to: &str, subject: &str, body: &str, attachment_path: Option<&str>) -> Result<String> {
        let mut mime_msg = format!(
            "To: {}\r\nSubject: {}\r\nContent-Type: multipart/mixed; boundary=\"boundary_marker\"\r\n\r\n",
            to, subject
        );

        // Body part
        mime_msg.push_str("--boundary_marker\r\n");
        mime_msg.push_str("Content-Type: text/plain; charset=\"UTF-8\"\r\n\r\n");
        mime_msg.push_str(body);
        mime_msg.push_str("\r\n\r\n");

        if let Some(path) = attachment_path {
            let path_obj = Path::new(path);
            let filename = path_obj.file_name().unwrap_or_default().to_string_lossy();
            let mime_type = from_path(path_obj).first_or_octet_stream();
            let content = fs::read(path).await.context("Failed to read attachment file")?;
            let encoded_content = BASE64_STANDARD.encode(content);

            mime_msg.push_str("--boundary_marker\r\n");
            mime_msg.push_str(&format!(
                "Content-Type: {}; name=\"{}\"\r\n",
                mime_type, filename
            ));
            mime_msg.push_str("Content-Transfer-Encoding: base64\r\n");
            mime_msg.push_str(&format!(
                "Content-Disposition: attachment; filename=\"{}\"\r\n\r\n",
                filename
            ));
            mime_msg.push_str(&encoded_content);
            mime_msg.push_str("\r\n\r\n");
        }

        mime_msg.push_str("--boundary_marker--\r\n");

        // Use upload method for sending raw MIME message
        // The API expects 'message/rfc822' for raw uploads
        let mime_type: mime::Mime = "message/rfc822".parse().unwrap();
        let cursor = std::io::Cursor::new(mime_msg.into_bytes());

        let (_resp, result_msg) = self.hub.users().messages_send(Message::default(), "me")
            .upload(cursor, mime_type)
            .await
            .context("Failed to send email via Gmail API")?;

        Ok(result_msg.id.unwrap_or_default())
    }
}
