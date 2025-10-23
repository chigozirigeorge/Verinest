//11
use super::sendmail::send_email;
use crate::{models::{
    verificationmodels::OtpPurpose,
    usermodel::VerificationStatus
}
};

pub async fn send_verification_email(
    to_email: &str,
    username: &str,
    token: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = "Email Verification";
    let template_path = "src/mail/templates/Verification-email.html";
    let base_url = "verinest.up.railway.app/api/auth/verify";
    let verification_link = create_verification_link(base_url, token);
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{verification_link}}".to_string(), verification_link)
    ];

    send_email(to_email, subject, template_path, &placeholders).await
}

fn create_verification_link(base_url: &str, token: &str) -> String {
    format!("{}?token={}", base_url, token)
}

pub async fn send_welcome_email(
    to_email: &str,
    username: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = "Welcome to Application";
    let template_path = "src/mail/templates/Welcome-email.html";
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string())
    ];

    send_email(to_email, subject, template_path, &placeholders).await
}

pub async fn send_forgot_password_email(
    to_email: &str,
    rest_link: &str,
    username: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = "Rest your Password";
    let template_path = "src/mail/templates/RestPassword-email.html";
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{rest_link}}".to_string(), rest_link.to_string())
    ];

    send_email(to_email, subject, template_path, &placeholders).await
}

pub async fn send_otp_email(
    to_email: &str,
    otp_code: &str,
    purpose: &OtpPurpose,
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = match purpose {
        OtpPurpose::AccountVerification => "Account Verification OTP",
        OtpPurpose::PasswordReset => "Password Reset OTP",
        OtpPurpose::Transaction => "Transaction Verification OTP",
        OtpPurpose::VerificationUpdate => "Verification Update OTP",
        OtpPurpose::SensitiveAction => "Security Verification OTP",
    };

    let template_path = "src/mail/templates/OTP-email.html";
    let placeholders = vec![
        ("{{otp_code}}".to_string(), otp_code.to_string()),
        ("{{purpose}}".to_string(), format!("{:?}", purpose)),
    ];

    send_email(to_email, subject, template_path, &placeholders).await
}

// In mails.rs - Add this function
pub async fn send_verification_status_email(
    to_email: &str,
    username: &str,
    status: &VerificationStatus,
    review_notes: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = match status {
        VerificationStatus::Approved => "Verification Approved",
        VerificationStatus::Rejected => "Verification Rejected", 
        VerificationStatus::Processing => "Verification Under Review",
        VerificationStatus::Submitted => "Verification Submitted",
        _ => "Verification Status Update",
    };

    let template_path = "src/mail/templates/Verification-Status.html";
    
    let status_display = match status {
        VerificationStatus::Unverified => "unverified",
        VerificationStatus::Approved => "Approved",
        VerificationStatus::Rejected => "Rejected",
        VerificationStatus::Processing => "Under Review",
        VerificationStatus::Submitted => "Submitted",
        VerificationStatus::Pending => "Pending",
        VerificationStatus::Expired => "Expired",
    };

    let app_url = std::env::var("APP_URL").unwrap_or_else(|_| "https://verinestorg.vercel.app/".to_string());
    let dashboard_url = format!("{}/dashboard", app_url);
    let verification_url = format!("{}/verification", app_url);

    let is_approved = status == &VerificationStatus::Approved;
    let is_rejected = status == &VerificationStatus::Rejected;
    let is_under_review = status == &VerificationStatus::Processing;

    let mut placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{status}}".to_string(), status_display.to_lowercase()),
        ("{{status_display}}".to_string(), status_display.to_string()),
        ("{{dashboard_url}}".to_string(), dashboard_url),
        ("{{verification_url}}".to_string(), verification_url),
        ("{{is_approved}}".to_string(), is_approved.to_string()),
        ("{{is_rejected}}".to_string(), is_rejected.to_string()),
        ("{{is_under_review}}".to_string(), is_under_review.to_string()),
    ];

    if let Some(notes) = review_notes {
        placeholders.push(("{{review_notes}}".to_string(), notes.to_string()));
    } else {
        placeholders.push(("{{review_notes}}".to_string(), "".to_string()));
    }

    send_email(to_email, subject, template_path, &placeholders).await
}

// Job / Application / Assignment Emails
pub async fn send_job_application_email(
    to_email: &str,
    username: &str,
    job_title: &str,
    applicant_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = "New Job Application Received";
    let template_path = "src/mail/templates/Job-Application.html";
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{job_title}}".to_string(), job_title.to_string()),
        ("{{applicant_name}}".to_string(), applicant_name.to_string()),
    ];

    send_email(to_email, subject, template_path, &placeholders).await
}

pub async fn send_job_assignment_email(
    to_email: &str,
    username: &str,
    job_title: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = "You've been assigned a job";
    let template_path = "src/mail/templates/Job-Assignment.html";
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{job_title}}".to_string(), job_title.to_string()),
    ];

    send_email(to_email, subject, template_path, &placeholders).await
}

pub async fn send_job_completion_email(
    to_email: &str,
    username: &str,
    job_title: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = "Job Completed";
    let template_path = "src/mail/templates/Job-Completion.html";
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{job_title}}".to_string(), job_title.to_string()),
    ];

    send_email(to_email, subject, template_path, &placeholders).await
}

// Transaction Emails
pub async fn send_payment_released_email(to_email: &str, username: &str, amount: f64) -> Result<(), Box<dyn std::error::Error>> {
    let subject = "Payment Released";
    let template_path = "src/mail/templates/Payment-Released.html";
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{amount}}".to_string(), format!("{:.2}", amount)),
    ];
    send_email(to_email, subject, template_path, &placeholders).await
}

pub async fn send_deposit_email(to_email: &str, username: &str, amount: f64, reference: &str) -> Result<(), Box<dyn std::error::Error>> {
    let subject = "Deposit Successful";
    let template_path = "src/mail/templates/Deposit.html";
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{amount}}".to_string(), format!("{:.2}", amount)),
        ("{{reference}}".to_string(), reference.to_string()),
    ];
    send_email(to_email, subject, template_path, &placeholders).await
}

pub async fn send_withdrawal_email(to_email: &str, username: &str, amount: f64, reference: &str) -> Result<(), Box<dyn std::error::Error>> {
    let subject = "Withdrawal Processed";
    let template_path = "src/mail/templates/Withdrawal.html";
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{amount}}".to_string(), format!("{:.2}", amount)),
        ("{{reference}}".to_string(), reference.to_string()),
    ];
    send_email(to_email, subject, template_path, &placeholders).await
}

pub async fn send_transfer_email(to_email: &str, username: &str, amount: f64, reference: &str, direction: &str) -> Result<(), Box<dyn std::error::Error>> {
    let subject = if direction == "sent" { "Transfer Sent" } else { "Transfer Received" };
    let template_path = "src/mail/templates/Transfer.html";
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{amount}}".to_string(), format!("{:.2}", amount)),
        ("{{reference}}".to_string(), reference.to_string()),
        ("{{direction}}".to_string(), direction.to_string()),
    ];
    send_email(to_email, subject, template_path, &placeholders).await
}