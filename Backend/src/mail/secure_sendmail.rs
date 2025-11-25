use std::fs;
use std::path::Path;
use serde_json::json;
use reqwest;
use ammonia::{Builder, UrlRelative};
use regex::Regex;
use tracing::{info, warn, error};

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;

// Rate limiting structure
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, Utc};

#[derive(Clone)]
pub struct RateLimiter {
    requests: Arc<Mutex<HashMap<String, Vec<DateTime<Utc>>>>>,
    max_requests: usize,
    window_minutes: i64,
}

impl RateLimiter {
    pub fn new(max_requests: usize, window_minutes: i64) -> Self {
        Self {
            requests: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window_minutes,
        }
    }

    pub fn is_allowed(&self, key: &str) -> bool {
        let mut requests = self.requests.lock().unwrap();
        let now = Utc::now();
        let window_start = now - chrono::Duration::minutes(self.window_minutes);

        let entry = requests.entry(key.to_string()).or_insert_with(Vec::new);
        entry.retain(|&timestamp| timestamp > window_start);

        if entry.len() < self.max_requests {
            entry.push(now);
            true
        } else {
            false
        }
    }
}

// HTML sanitizer for email content
fn sanitize_html(input: &str) -> String {
    let mut builder = Builder::default();
    
    // Allow safe HTML tags for emails
    builder
        .add_tags(&["p", "br", "strong", "em", "u", "span", "div", "a", "h1", "h2", "h3", "h4", "h5", "h6"])
        .add_generic_attributes(&["style", "class"])
        .add_tag_attributes("a", &["href", "target"])
        .add_tag_attributes("p", &["style"])
        .add_tag_attributes("div", &["style"])
        .add_tag_attributes("span", &["style"])
        .url_relative(UrlRelative::PassThrough)
        .link_rel(None);

    builder.clean(input).to_string()
}

// Validate template path to prevent path traversal
fn validate_template_path(template_path: &str) -> Result<(), String> {
    let base_path = Path::new("src/mail/templates");
    let full_path = Path::new(template_path);
    
    // Check if the path is within the allowed directory
    if !full_path.starts_with(base_path) {
        return Err("Invalid template path: path traversal detected".to_string());
    }

    // Check if file exists and has .html extension
    if !full_path.exists() {
        return Err("Template file not found".to_string());
    }

    if full_path.extension() != Some(std::ffi::OsStr::new("html")) {
        return Err("Template must be an HTML file".to_string());
    }

    Ok(())
}

// Improved email validation with regex
fn validate_email(email: &str) -> Result<(), String> {
    let email_regex = Regex::new(
        r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"
    ).map_err(|_| "Invalid regex pattern".to_string())?;

    if email_regex.is_match(email) {
        Ok(())
    } else {
        Err("Invalid email address format".to_string())
    }
}

// Sanitize error messages to prevent information disclosure
fn sanitize_error(error: &str, context: &str) -> String {
    // Log the full error for debugging
    error!("Full error in {}: {}", context, error);
    
    // Return a generic error message to users
    match context {
        "template" => "Failed to process email template".to_string(),
        "network" => "Network error occurred while sending email".to_string(),
        "auth" => "Email service authentication failed".to_string(),
        "validation" => "Invalid input provided".to_string(),
        _ => "An error occurred while sending email".to_string(),
    }
}

// Audit logging for email operations
fn log_email_operation(to_email: &str, subject: &str, template_path: &str, success: bool, error: Option<&str>) {
    let log_entry = json!({
        "timestamp": Utc::now().to_rfc3339(),
        "operation": "email_send",
        "to_email": to_email,
        "subject": subject,
        "template": template_path,
        "success": success,
        "error": error
    });

    if success {
        info!("Email sent successfully: {}", log_entry);
    } else {
        error!("Email send failed: {}", log_entry);
    }
}

pub async fn send_email(
    to_email: &str,
    subject: &str,
    template_path: &str,
    placeholders: &[(String, String)]
) -> Result<(), Box<dyn std::error::Error>> {
    // Initialize rate limiter (10 emails per minute per recipient)
    static RATE_LIMITER: std::sync::LazyLock<RateLimiter> = std::sync::LazyLock::new(|| RateLimiter::new(5, 1));
    
    // Rate limiting check
    if !RATE_LIMITER.is_allowed(to_email) {
        return Err("Rate limit exceeded for email sending".into());
    }

    // Validate inputs
    if to_email.is_empty() {
        return Err("Email recipient cannot be empty".into());
    }

    validate_email(to_email).map_err(|e| e.to_string())?;
    validate_template_path(template_path).map_err(|e| e.to_string())?;

    // Read and process template
    let mut html_template = match fs::read_to_string(template_path) {
        Ok(content) => content,
        Err(e) => {
            let sanitized_error = sanitize_error(&e.to_string(), "template");
            log_email_operation(to_email, subject, template_path, false, Some(&sanitized_error));
            return Err(sanitized_error.into());
        }
    };

    // Sanitize all placeholder values to prevent HTML injection
    let sanitized_placeholders: Vec<(String, String)> = placeholders
        .iter()
        .map(|(key, value)| {
            let sanitized_value = sanitize_html(value);
            (key.clone(), sanitized_value)
        })
        .collect();

    // Replace placeholders with sanitized values
    for (key, value) in &sanitized_placeholders {
        html_template = html_template.replace(key, value);
    }

    // Send with both Resend and SMTP fallback
    match send_with_fallback(to_email, subject, &html_template).await {
        Ok(_) => {
            log_email_operation(to_email, subject, template_path, true, None);
            Ok(())
        }
        Err(e) => {
            let sanitized_error = sanitize_error(&e.to_string(), "network");
            log_email_operation(to_email, subject, template_path, false, Some(&sanitized_error));
            
            // Even if email fails, we still want to continue with notification
            warn!("Email sending failed but continuing with notification: {}", sanitized_error);
            Err(sanitized_error.into())
        }
    }
}

async fn send_with_fallback(
    to_email: &str,
    subject: &str,
    html_body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut last_error = None;

    // Try Resend first
    match send_via_resend(to_email, subject, html_body).await {
        Ok(_) => {
            info!("Email sent successfully via Resend to {}", to_email);
            return Ok(());
        }
        Err(e) => {
            warn!("Resend failed, trying SMTP fallback: {}", e);
            last_error = Some(e);
        }
    }

    // Fallback to SMTP
    match send_via_smtp(to_email, subject, html_body).await {
        Ok(_) => {
            info!("Email sent successfully via SMTP to {}", to_email);
            Ok(())
        }
        Err(e) => {
            error!("Both Resend and SMTP failed for {}: {}", to_email, e);
            Err(last_error.unwrap_or(e.to_string()).into())
        }
    }
}

async fn send_via_resend(
    to_email: &str,
    subject: &str,
    html_body: &str,
) -> Result<String, String> {
    let resend_api_key = std::env::var("RESEND_API_KEY")
        .map_err(|_| "Email service configuration error".to_string())?;

    let from_email = std::env::var("FROM_EMAIL")
        .unwrap_or_else(|_| "Verinest <noreply@verinest.xyz>".to_string());

    // Validate API key format
    if resend_api_key.is_empty() {
        return Err("Email service configuration error".to_string());
    }

    let client = reqwest::Client::new();
    let request_body = json!({
        "from": from_email,
        "to": to_email,
        "subject": subject,
        "html": html_body,
    });

    let response = client
        .post("https://api.resend.com/emails")
        .header("Authorization", format!("Bearer {}", resend_api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| sanitize_error(&e.to_string(), "network"))?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .unwrap_or_else(|_| "No response body".to_string());

    if status.is_success() {
        // Extract email ID from response
        if let Ok(body) = serde_json::from_str::<serde_json::Value>(&response_text) {
            if let Some(id) = body.get("id").and_then(|v| v.as_str()) {
                return Ok(id.to_string());
            }
        }
        Ok("success".to_string())
    } else {
        Err(sanitize_error(&format!("HTTP {}: {}", status.as_u16(), response_text), "auth"))
    }
}

async fn send_via_smtp(
    to_email: &str,
    subject: &str,
    html_body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use lettre::{
        Message, SmtpTransport, Transport,
        message::{header::ContentType, MultiPart, SinglePart},
        transport::smtp::authentication::Credentials,
    };

    let smtp_host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_string());
    let smtp_username = std::env::var("SMTP_USERNAME").unwrap_or_else(|_| "".to_string());
    let smtp_password = std::env::var("SMTP_PASSWORD").unwrap_or_else(|_| "".to_string());
    let smtp_port: u16 = std::env::var("SMTP_PORT")
        .unwrap_or_else(|_| "587".to_string())
        .parse()
        .unwrap_or(587);

    let from_email = std::env::var("FROM_EMAIL")
        .unwrap_or_else(|_| "Verinest <noreply@verinest.xyz>".to_string());

    // Create email message
    let email = Message::builder()
        .from(from_email.parse()?)
        .to(to_email.parse()?)
        .subject(subject)
        .multipart(
            MultiPart::alternative()
                .singlepart(
                    SinglePart::builder()
                        .header(ContentType::TEXT_HTML)
                        .body(html_body.to_string())
                )
        )?;

    // Create SMTP transport
    let creds = Credentials::new(smtp_username, smtp_password);
    let mailer = if smtp_port == 465 {
        SmtpTransport::relay(&smtp_host)?
            .port(smtp_port)
            .credentials(creds)
            .build()
    } else {
        SmtpTransport::relay(&smtp_host)?
            .port(smtp_port)
            .credentials(creds)
            .build()
    };

    // Send the email
    match mailer.send(&email) {
        Ok(_) => {
            info!("Email sent successfully via SMTP to {}", to_email);
            Ok(())
        }
        Err(e) => {
            error!("SMTP send failed: {}", e);
            Err(format!("SMTP send failed: {}", sanitize_error(&e.to_string(), "network")).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        assert!(validate_email("test@example.com").is_ok());
        assert!(validate_email("user.name+tag@domain.co.uk").is_ok());
        assert!(validate_email("invalid-email").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("test@.com").is_err());
    }

    #[test]
    fn test_html_sanitization() {
        let input = "<script>alert('xss')</script><p>Safe content</p>";
        let sanitized = sanitize_html(input);
        assert!(!sanitized.contains("<script>"));
        assert!(sanitized.contains("<p>Safe content</p>"));
    }

    #[test]
    fn test_template_path_validation() {
        assert!(validate_template_path("src/mail/templates/test.html").is_ok());
        assert!(validate_template_path("../../../etc/passwd").is_err());
        assert!(validate_template_path("src/mail/templates/../config.txt").is_err());
    }
}
