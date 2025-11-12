//11
use super::sendmail::send_email;
use crate::{models::{
    verificationmodels::OtpPurpose,
    usermodel::VerificationStatus
}
};

/// Send verification email with proper error handling
pub async fn send_verification_email(
    to_email: &str,
    username: &str,
    token: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = "Email Verification";
    let template_path = "src/mail/templates/Verification-email.html";
    let base_url = "https://verinest.up.railway.app/api/auth/verify";
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

// In mails.rs - Add progress update email function

pub async fn send_progress_update_email(
    to_email: &str,
    username: &str,
    job_title: &str,
    progress_percentage: i32,
    progress_description: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = format!("Progress Update: {}", job_title);
    let template_path = "src/mail/templates/Progress-Update.html";
    
    let app_url = std::env::var("APP_URL")
        .unwrap_or_else(|_| "https://verinestorg.vercel.app".to_string());
    let dashboard_url = format!("{}/dashboard", app_url);
    
    let update_date = chrono::Utc::now().format("%B %d, %Y at %I:%M %p").to_string();
    
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{job_title}}".to_string(), job_title.to_string()),
        ("{{progress_percentage}}".to_string(), progress_percentage.to_string()),
        ("{{progress_description}}".to_string(), progress_description.to_string()),
        ("{{update_date}}".to_string(), update_date),
        ("{{dashboard_url}}".to_string(), dashboard_url),
    ];

    send_email(to_email, &subject, template_path, &placeholders).await
}

pub async fn send_dispute_notification_email(
    to_email: &str,
    username: &str,
    dispute_reason: &str,
    job_title: &str,
    is_raised_by: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = if is_raised_by {
        "Dispute Created - Under Review"
    } else {
        "Dispute Raised Against You"
    };
    
    let template_path = "src/mail/templates/Dispute-Notification.html";
    
    let app_url = std::env::var("APP_URL")
        .unwrap_or_else(|_| "https://verinestorg.vercel.app".to_string());
    let disputes_url = format!("{}/disputes", app_url);
    
    let message = if is_raised_by {
        format!("Your dispute has been created and is being reviewed by our team. We will notify you once a resolution has been reached.")
    } else {
        format!("A dispute has been raised against you regarding the job: {}. Please review the details and provide your response.", job_title)
    };
    
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{dispute_reason}}".to_string(), dispute_reason.to_string()),
        ("{{job_title}}".to_string(), job_title.to_string()),
        ("{{message}}".to_string(), message),
        ("{{disputes_url}}".to_string(), disputes_url),
    ];

    send_email(to_email, subject, template_path, &placeholders).await
}

pub async fn send_contract_proposal_email(
    to_email: &str,
    username: &str,
    proposer_name: &str,
    job_title: &str,
    agreed_rate: f64,
    agreed_timeline: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = format!("Contract Proposal: {}", job_title);
    let template_path = "src/mail/templates/Contract-Proposal.html";
    
    let app_url = std::env::var("APP_URL")
        .unwrap_or_else(|_| "https://verinestorg.vercel.app".to_string());
    let chat_url = format!("{}/chat", app_url);
    
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{proposer_name}}".to_string(), proposer_name.to_string()),
        ("{{job_title}}".to_string(), job_title.to_string()),
        ("{{agreed_rate}}".to_string(), format!("₦{:.2}", agreed_rate)),
        ("{{agreed_timeline}}".to_string(), format!("{} days", agreed_timeline)),
        ("{{chat_url}}".to_string(), chat_url),
    ];

    send_email(to_email, &subject, template_path, &placeholders).await
}

pub async fn send_new_message_notification_email(
    to_email: &str,
    username: &str,
    sender_name: &str,
    message_preview: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = format!("New message from {}", sender_name);
    let template_path = "src/mail/templates/New-Message.html";
    
    let app_url = std::env::var("APP_URL")
        .unwrap_or_else(|_| "https://verinestorg.vercel.app".to_string());
    let chat_url = format!("{}/chat", app_url);
    
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{sender_name}}".to_string(), sender_name.to_string()),
        ("{{message_preview}}".to_string(), message_preview.to_string()),
        ("{{chat_url}}".to_string(), chat_url),
    ];

    send_email(to_email, &subject, template_path, &placeholders).await
}


pub async fn send_contract_signature_otp_email(
    to_email: &str,
    username: &str,
    otp_code: &str,
    agreed_rate: &f64,
    agreed_timeline: i32,
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = "Contract Signature Verification";
    let template_path = "src/mail/templates/Contract-Signature-OTP.html";
    
    let app_url = std::env::var("APP_URL")
        .unwrap_or_else(|_| "https://verinestorg.vercel.app".to_string());
    
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{otp_code}}".to_string(), otp_code.to_string()),
        ("{{agreed_rate}}".to_string(), format!("₦{:.2}", agreed_rate)),
        ("{{agreed_timeline}}".to_string(), format!("{} days", agreed_timeline)),
        ("{{app_url}}".to_string(), app_url),
    ];

    send_email(to_email, subject, template_path, &placeholders).await
}

pub async fn send_service_inquiry_email(
    to_email: &str,
    username: &str,
    inquirer_name: &str,
    service_title: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let subject = "New Inquiry for Your Service";
    let template_path = "src/mail/templates/Service-Inquiry.html";
    
    let app_url = std::env::var("APP_URL")
        .unwrap_or_else(|_| "https://verinestorg.vercel.app".to_string());
    let inquiries_url = format!("{}/vendor/inquiries", app_url);
    
    let placeholders = vec![
        ("{{username}}".to_string(), username.to_string()),
        ("{{inquirer_name}}".to_string(), inquirer_name.to_string()),
        ("{{service_title}}".to_string(), service_title.to_string()),
        ("{{inquiries_url}}".to_string(), inquiries_url),
    ];

    send_email(to_email, subject, template_path, &placeholders).await
}