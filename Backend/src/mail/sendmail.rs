use std::fs;
use serde_json::json;
use reqwest;
use tokio::time::{sleep, Duration};

const MAX_RETRIES: u32 = 3;
const RETRY_DELAY_MS: u64 = 1000;

pub async fn send_email(
    to_email: &str,
    subject: &str,
    template_path: &str,
    placeholders: &[(String, String)]
) -> Result<(), Box<dyn std::error::Error>> {
    // Validate inputs
    if to_email.is_empty() {
        return Err("Email recipient cannot be empty".into());
    }
    if !to_email.contains('@') {
        return Err(format!("Invalid email address: {}", to_email).into());
    }

    // Read and process template
    let mut html_template = match fs::read_to_string(template_path) {
        Ok(content) => content,
        Err(e) => {
            tracing::error!("Failed to read email template {}: {}", template_path, e);
            return Err(format!("Template not found: {}", template_path).into());
        }
    };

    for (key, value) in placeholders {
        html_template = html_template.replace(key, value);
    }

    // Send with retries
    send_with_retries(to_email, subject, &html_template).await
}

async fn send_with_retries(
    to_email: &str,
    subject: &str,
    html_body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut last_error = None;

    for attempt in 1..=MAX_RETRIES {
        match send_via_resend(to_email, subject, html_body).await {
            Ok(email_id) => {
                tracing::info!(
                    "✓ Email sent successfully to {} (id: {})",
                    to_email,
                    email_id
                );
                return Ok(());
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < MAX_RETRIES {
                    let delay = RETRY_DELAY_MS * (2_u64.pow(attempt - 1)); // Exponential backoff
                    tracing::warn!(
                        "Email send attempt {} failed for {}. Retrying in {}ms...",
                        attempt,
                        to_email,
                        delay
                    );
                    sleep(Duration::from_millis(delay)).await;
                }
            }
        }
    }

    let error_msg = last_error
        .map(|e| format!("Failed after {} retries: {}", MAX_RETRIES, e))
        .unwrap_or_else(|| "Unknown email sending error".to_string());

    tracing::error!("✗ Email failed for {}: {}", to_email, error_msg);
    Err(error_msg.into())
}

async fn send_via_resend(
    to_email: &str,
    subject: &str,
    html_body: &str,
) -> Result<String, String> {
    let resend_api_key = std::env::var("RESEND_API_KEY")
        .map_err(|_| "RESEND_API_KEY environment variable not set".to_string())?;

    let from_email = std::env::var("FROM_EMAIL")
        .unwrap_or_else(|_| "Verinest <noreply@verinest.com>".to_string());

    // Validate API key format
    if resend_api_key.is_empty() {
        return Err("RESEND_API_KEY is empty".to_string());
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
        .map_err(|e| format!("Network error: {}", e))?;

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
        Err(format!(
            "Resend API error ({}): {}",
            status.as_u16(),
            response_text
        ))
    }
}