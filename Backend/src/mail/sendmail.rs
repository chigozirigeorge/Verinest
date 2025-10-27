//10
use std::{env, fs};
use lettre::{
    message::{header, SinglePart},
    transport::smtp::authentication::Credentials,
    Message, SmtpTransport,
    Transport,
};

pub async fn send_email(
    to_email: &str,
    subject: &str,
    template_path: &str,
    placeholders: &[(String, String)]
) -> Result<(), Box<dyn std::error::Error>> {
    let smtp_username = env::var("SMTP_USERNAME")?;
    let smtp_password = env::var("SMTP_PASSWORD")?;
    let smtp_server = env::var("SMTP_SERVER")?;
    let smtp_port: u16 = env::var("SMTP_PORT")?.parse()?;

    let mut html_template = fs::read_to_string(template_path)?;

    for (key, value) in placeholders {
        html_template = html_template.replace(key, value)
    }

    let email = Message::builder()
        .from(smtp_username.parse()?)
        .to(to_email.parse()?)
        .subject(subject)
        .header(header::ContentType::TEXT_HTML)
        .singlepart(SinglePart::builder()
            .header(header::ContentType::TEXT_HTML)
            .body(html_template)
        )?;

    let creds = Credentials::new(smtp_username.clone(), smtp_password.clone());
    let mailer = SmtpTransport::starttls_relay(&smtp_server)?
        .credentials(creds)
        .port(smtp_port)
        .build();
    
    let result = mailer.send(&email);

    match result {
        Ok(_) => println!("Email sent successfully!"),
        Err(e) => println!("Failed to send email: {:?}", e),
    }

    Ok(())
}


// // sendmail.rs
// use std::fs;
// use serde_json::json;
// use reqwest;

// pub async fn send_email(
//     to_email: &str,
//     subject: &str,
//     template_path: &str,
//     placeholders: &[(String, String)]
// ) -> Result<(), Box<dyn std::error::Error>> {
//     let resend_api_key = std::env::var("RESEND_API_KEY")?;
//     let from_email = std::env::var("FROM_EMAIL").unwrap_or_else(|_| "Verinest <noreply@verinest.com>".to_string());

//     // Read and process template
//     let mut html_template = fs::read_to_string(template_path)?;
//     for (key, value) in placeholders {
//         html_template = html_template.replace(key, value);
//     }

//     // Prepare Resend API request
//     let client = reqwest::Client::new();
//     let response = client
//         .post("https://api.resend.com/emails")
//         .header("Authorization", format!("Bearer {}", resend_api_key))
//         .header("Content-Type", "application/json")
//         .json(&json!({
//             "from": from_email,
//             "to": to_email,
//             "subject": subject,
//             "html": html_template,
//         }))
//         .send()
//         .await?;

//     if response.status().is_success() {
//         println!("Email sent successfully via Resend!");
//     } else {
//         let error_text = response.text().await?;
//         println!("Failed to send email via Resend: {}", error_text);
//         return Err(format!("Resend API error: {}", error_text).into());
//     }

//     Ok(())
// }