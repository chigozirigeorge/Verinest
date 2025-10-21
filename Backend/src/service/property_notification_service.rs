// src/service/notification_service.rs
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, Duration};
use uuid::Uuid;

use crate::models::usermodel::User;
use crate::models::propertymodel::Property;

// Notification types and channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Email,
    SMS,
    InApp,
    Push,
    WhatsApp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    PropertyAssigned,
    VerificationPending,
    VerificationApproved,
    VerificationRejected,
    PropertyLive,
    DocumentRequired,
    PaymentDue,
    SystemAlert,
    MarketingPromo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationPriority {
    Low,
    Medium,
    High,
    Critical,
}

// Notification template structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTemplate {
    pub id: String,
    pub notification_type: NotificationType,
    pub channel: NotificationChannel,
    pub subject_template: String,
    pub body_template: String,
    pub variables: Vec<String>,
    pub priority: NotificationPriority,
    pub expiry_hours: Option<u32>,
}

// Individual notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub notification_type: NotificationType,
    pub channel: NotificationChannel,
    pub priority: NotificationPriority,
    pub subject: String,
    pub body: String,
    pub data: HashMap<String, String>,
    pub scheduled_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
    pub read_at: Option<DateTime<Utc>>,
    pub status: NotificationStatus,
    pub retry_count: u32,
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationStatus {
    Pending,
    Sent,
    Delivered,
    Read,
    Failed,
    Expired,
}

// External service configurations
#[derive(Debug, Clone)]
pub struct EmailConfig {
    pub smtp_server: String,
    pub smtp_port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub from_name: String,
}

#[derive(Debug, Clone)]
pub struct SmsConfig {
    pub provider: String, // "twilio", "termii", "bulk_sms_nigeria"
    pub api_key: String,
    pub sender_id: String,
    pub webhook_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WhatsAppConfig {
    pub business_account_id: String,
    pub access_token: String,
    pub phone_number_id: String,
    pub webhook_verify_token: String,
}

// Custom errors
#[derive(Debug)]
pub enum NotificationError {
    TemplateNotFound(String),
    InvalidChannel(String),
    ExternalServiceError(String),
    NetworkError(String),
    ValidationError(String),
    RateLimitExceeded(String),
}

impl fmt::Display for NotificationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NotificationError::TemplateNotFound(msg) => write!(f, "Template not found: {}", msg),
            NotificationError::InvalidChannel(msg) => write!(f, "Invalid channel: {}", msg),
            NotificationError::ExternalServiceError(msg) => write!(f, "External service error: {}", msg),
            NotificationError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            NotificationError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            NotificationError::RateLimitExceeded(msg) => write!(f, "Rate limit exceeded: {}", msg),
        }
    }
}

impl Error for NotificationError {}

// Main notification service
pub struct NotificationService {
    client: Client,
    templates: HashMap<String, NotificationTemplate>,
    email_config: EmailConfig,
    sms_config: SmsConfig,
    whatsapp_config: Option<WhatsAppConfig>,
    rate_limiter: HashMap<String, Vec<DateTime<Utc>>>,
}

impl NotificationService {
    pub fn new(
        email_config: EmailConfig,
        sms_config: SmsConfig,
        whatsapp_config: Option<WhatsAppConfig>,
    ) -> Self {
        let mut service = Self {
            client: Client::new(),
            templates: HashMap::new(),
            email_config,
            sms_config,
            whatsapp_config,
            rate_limiter: HashMap::new(),
        };

        // Initialize default templates
        service.initialize_default_templates();
        service
    }

    /// Initialize default notification templates
    fn initialize_default_templates(&mut self) {
        // Agent Property Assignment Template
        self.templates.insert("agent_property_assigned_email".to_string(), NotificationTemplate {
            id: "agent_property_assigned_email".to_string(),
            notification_type: NotificationType::PropertyAssigned,
            channel: NotificationChannel::Email,
            subject_template: "New Property Verification Assignment - {property_title}".to_string(),
            body_template: r#"
Dear {agent_name},

You have been assigned to verify a new property on VeriNest:

Property Details:
- Title: {property_title}
- Address: {property_address}
- Property Type: {property_type}
- Landlord: {landlord_name}
- Contact: {landlord_phone}

Verification Requirements:
1. Visit the property at the provided address
2. Verify the property exists and matches the description
3. Take verification photos from multiple angles
4. Compare with landlord-provided photos
5. Submit your verification report within 48 hours

Property Photos: {property_photos}
Landlord Documents: {document_links}

To complete this verification, please log into your VeriNest dashboard:
https://verinest.vercel.app/agent/verifications

Deadline: {verification_deadline}
Property Reference: {property_reference}

If you have any questions, please contact our support team.

Best regards,
VeriNest Team
            "#.to_string(),
            variables: vec![
                "agent_name".to_string(), "property_title".to_string(), "property_address".to_string(),
                "property_type".to_string(), "landlord_name".to_string(), "landlord_phone".to_string(),
                "property_photos".to_string(), "document_links".to_string(), "verification_deadline".to_string(),
                "property_reference".to_string()
            ],
            priority: NotificationPriority::High,
            expiry_hours: Some(72),
        });

        // Lawyer Document Verification Template
        self.templates.insert("lawyer_documents_ready_email".to_string(), NotificationTemplate {
            id: "lawyer_documents_ready_email".to_string(),
            notification_type: NotificationType::DocumentRequired,
            channel: NotificationChannel::Email,
            subject_template: "Document Verification Required - {property_title}".to_string(),
            body_template: r#"
Dear {lawyer_name},

A property has been agent-verified and is now ready for legal document review:

Property Details:
- Title: {property_title}
- Address: {property_address}
- Property Type: {property_type}
- Listing Type: {listing_type}
- Price: ₦{property_price:,}
- Landlord: {landlord_name}

Agent Verification Summary:
- Verification Status: {agent_status}
- Agent Notes: {agent_notes}
- Verification Date: {agent_verification_date}

Documents to Review:
{document_list}

Agent Verification Photos: {agent_photos}

Please review all documents for:
1. Authenticity and validity
2. Proper ownership documentation
3. Government approvals and permits
4. Compliance with local regulations
5. Any legal encumbrances

Access the documents and submit your review:
https://verinest.vercel.app/lawyer/verifications

Deadline: {verification_deadline}
Property Reference: {property_reference}

For any clarifications, contact our legal support team.

Best regards,
VeriNest Legal Team
            "#.to_string(),
            variables: vec![
                "lawyer_name".to_string(), "property_title".to_string(), "property_address".to_string(),
                "property_type".to_string(), "listing_type".to_string(), "property_price".to_string(),
                "landlord_name".to_string(), "agent_status".to_string(), "agent_notes".to_string(),
                "agent_verification_date".to_string(), "document_list".to_string(), "agent_photos".to_string(),
                "verification_deadline".to_string(), "property_reference".to_string()
            ],
            priority: NotificationPriority::High,
            expiry_hours: Some(96),
        });

        // Property Approved - Live Notification
        self.templates.insert("property_live_email".to_string(), NotificationTemplate {
            id: "property_live_email".to_string(),
            notification_type: NotificationType::PropertyLive,
            channel: NotificationChannel::Email,
            subject_template: "🎉 Your Property is Now Live on VeriNest!".to_string(),
            body_template: r#"
Dear {landlord_name},

Congratulations! Your property has been successfully verified and is now live on VeriNest:

Property: {property_title}
Address: {property_address}
Reference: {property_reference}
Listed Price: ₦{property_price:,}

Your property listing includes:
- Agent-verified location and condition
- Lawyer-verified legal documents
- Professional photos and description
- Premium verification badge

Property URL: https://verinest.vercel.app/properties/{property_id}

Next Steps:
1. Share your property link with potential buyers/tenants
2. Monitor inquiries in your dashboard
3. Keep your contact information updated
4. Consider premium listing features for more visibility

Dashboard: https://verinest.vercel.app/landlord/properties
Property Analytics: https://verinest.vercel.app/landlord/analytics/{property_id}

Tips for Success:
- Respond quickly to inquiries (within 2 hours for best results)
- Keep property information up to date
- Consider professional staging for viewings
- Price competitively based on market data

Your property scored {verification_score}/100 in our verification process.
Market Analysis: {market_analysis}

Thank you for choosing VeriNest - Nigeria's most trusted property platform!

Best regards,
The VeriNest Team

P.S. Refer other landlords and earn VERN tokens: https://verinest.vercel.app/referral
            "#.to_string(),
            variables: vec![
                "landlord_name".to_string(), "property_title".to_string(), "property_address".to_string(),
                "property_reference".to_string(), "property_price".to_string(), "property_id".to_string(),
                "verification_score".to_string(), "market_analysis".to_string()
            ],
            priority: NotificationPriority::Medium,
            expiry_hours: Some(168), // 1 week
        });

        // Property Rejection Notification
        self.templates.insert("property_rejected_email".to_string(), NotificationTemplate {
            id: "property_rejected_email".to_string(),
            notification_type: NotificationType::VerificationRejected,
            channel: NotificationChannel::Email,
            subject_template: "Property Verification Update - {property_title}".to_string(),
            body_template: r#"
Dear {landlord_name},

We regret to inform you that your property listing has not passed our verification process:

Property: {property_title}
Address: {property_address}
Reference: {property_reference}
Rejection Stage: {rejection_stage}

Reason for Rejection:
{rejection_reason}

Specific Issues Identified:
{rejection_details}

What You Can Do:
1. Address the issues mentioned above
2. Upload corrected documents or information
3. Resubmit your property for verification

Common Solutions:
- Ensure all documents are valid and current
- Verify property address matches legal documents
- Provide clear, high-quality property photos
- Ensure property description is accurate

Resubmission Process:
1. Log into your dashboard: https://verinest.vercel.app/landlord
2. Edit your property listing
3. Upload corrected documents/photos
4. Resubmit for verification

Need Help?
- Document Issues: legal@verinest.com
- Technical Support: support@verinest.com
- Phone: +234-800-VERINEST

We're here to help you get your property verified and listed successfully.

Best regards,
VeriNest Support Team
            "#.to_string(),
            variables: vec![
                "landlord_name".to_string(), "property_title".to_string(), "property_address".to_string(),
                "property_reference".to_string(), "rejection_stage".to_string(), "rejection_reason".to_string(),
                "rejection_details".to_string()
            ],
            priority: NotificationPriority::High,
            expiry_hours: Some(72),
        });

        // SMS Templates
        self.templates.insert("agent_assignment_sms".to_string(), NotificationTemplate {
            id: "agent_assignment_sms".to_string(),
            notification_type: NotificationType::PropertyAssigned,
            channel: NotificationChannel::SMS,
            subject_template: "".to_string(),
            body_template: "VeriNest: New property verification assigned. {property_title} at {property_address}. Deadline: {deadline}. Check your dashboard for details.".to_string(),
            variables: vec!["property_title".to_string(), "property_address".to_string(), "deadline".to_string()],
            priority: NotificationPriority::High,
            expiry_hours: Some(48),
        });

        self.templates.insert("verification_approved_sms".to_string(), NotificationTemplate {
            id: "verification_approved_sms".to_string(),
            notification_type: NotificationType::VerificationApproved,
            channel: NotificationChannel::SMS,
            subject_template: "".to_string(),
            body_template: "🎉 VeriNest: Your property '{property_title}' has been verified and is now LIVE! Share: https://verinest.vercel.app/p/{property_id}".to_string(),
            variables: vec!["property_title".to_string(), "property_id".to_string()],
            priority: NotificationPriority::Medium,
            expiry_hours: Some(24),
        });
    }

    /// Send notification to agent when property is assigned
    pub async fn notify_agent_property_assigned(
        &mut self,
        agent: &User,
        property: &Property,
    ) -> Result<Vec<Uuid>, NotificationError> {
        let mut notification_ids = Vec::new();

        // Prepare template variables
        let mut variables = HashMap::new();
        variables.insert("agent_name".to_string(), agent.name.clone());
        variables.insert("property_title".to_string(), property.title.clone());
        variables.insert("property_address".to_string(), property.address.clone());
        variables.insert("property_type".to_string(), format!("{:?}", property.property_type));
        variables.insert("landlord_name".to_string(), "Property Owner".to_string()); // Would get from landlord lookup
        variables.insert("landlord_phone".to_string(), "+234-XXX-XXX-XXXX".to_string()); // Would get from landlord
        variables.insert("property_photos".to_string(), property.property_photos.0.join(", "));
        variables.insert("document_links".to_string(), "Available in dashboard".to_string());
        variables.insert("verification_deadline".to_string(), 
            (Utc::now() + Duration::hours(48)).format("%Y-%m-%d %H:%M UTC").to_string());
        variables.insert("property_reference".to_string(), format!("VN-{}", property.id.to_string().chars().take(8).collect::<String>()));

        // Send email notification
        if let Some(email) = &agent.email {
            let email_id = self.send_notification(
                agent.id,
                "agent_property_assigned_email",
                variables.clone(),
                Some(email.clone()),
            ).await?;
            notification_ids.push(email_id);
        }

        // Send SMS notification
        if let Some(phone) = &agent.verification_number {
            let sms_id = self.send_notification(
                agent.id,
                "agent_assignment_sms",
                variables.clone(),
                Some(phone.clone()),
            ).await?;
            notification_ids.push(sms_id);
        }

        println!("📧 Notifications sent to agent {} for property assignment", agent.username);
        Ok(notification_ids)
    }

    /// Send notification to lawyer when documents are ready
    pub async fn notify_lawyer_documents_ready(
        &mut self,
        lawyer: &User,
        property: &Property,
        agent_notes: &str,
    ) -> Result<Vec<Uuid>, NotificationError> {
        let mut notification_ids = Vec::new();

        // Prepare template variables
        let mut variables = HashMap::new();
        variables.insert("lawyer_name".to_string(), lawyer.name.clone());
        variables.insert("property_title".to_string(), property.title.clone());
        variables.insert("property_address".to_string(), property.address.clone());
        variables.insert("property_type".to_string(), format!("{:?}", property.property_type));
        variables.insert("listing_type".to_string(), format!("{:?}", property.listing_type));
        variables.insert("property_price".to_string(), property.price.to_string());
        variables.insert("landlord_name".to_string(), "Property Owner".to_string());
        variables.insert("agent_status".to_string(), "Approved".to_string());
        variables.insert("agent_notes".to_string(), agent_notes.to_string());
        variables.insert("agent_verification_date".to_string(), 
            property.agent_verified_at.unwrap_or(Utc::now()).format("%Y-%m-%d").to_string());
        
        // Build document list
        let mut documents = Vec::new();
        if property.certificate_of_occupancy.is_some() {
            documents.push("✓ Certificate of Occupancy");
        }
        if property.deed_of_assignment.is_some() {
            documents.push("✓ Deed of Assignment");
        }
        if property.survey_plan.is_some() {
            documents.push("✓ Survey Plan");
        }
        if property.building_plan_approval.is_some() {
            documents.push("✓ Building Plan Approval");
        }
        variables.insert("document_list".to_string(), documents.join("\n"));
        
        variables.insert("agent_photos".to_string(), 
            property.agent_verification_photos.as_ref()
                .map(|photos| photos.0.join(", "))
                .unwrap_or_else(|| "None".to_string()));
        variables.insert("verification_deadline".to_string(), 
            (Utc::now() + Duration::hours(96)).format("%Y-%m-%d %H:%M UTC").to_string());
        variables.insert("property_reference".to_string(), 
            format!("VN-{}", property.id.to_string().chars().take(8).collect::<String>()));

        // Send email notification
        if let Some(email) = &lawyer.email {
            let email_id = self.send_notification(
                lawyer.id,
                "lawyer_documents_ready_email",
                variables.clone(),
                Some(email.clone()),
            ).await?;
            notification_ids.push(email_id);
        }

        println!("📧 Document review notification sent to lawyer {}", lawyer.username);
        Ok(notification_ids)
    }

    /// Send notification when property goes live
    pub async fn notify_property_live(
        &mut self,
        landlord: &User,
        property: &Property,
        verification_score: u32,
        market_analysis: &str,
    ) -> Result<Vec<Uuid>, NotificationError> {
        let mut notification_ids = Vec::new();

        // Prepare template variables
        let mut variables = HashMap::new();
        variables.insert("landlord_name".to_string(), landlord.name.clone());
        variables.insert("property_title".to_string(), property.title.clone());
        variables.insert("property_address".to_string(), property.address.clone());
        variables.insert("property_reference".to_string(), 
            format!("VN-{}", property.id.to_string().chars().take(8).collect::<String>()));
        variables.insert("property_price".to_string(), property.price.to_string());
        variables.insert("property_id".to_string(), property.id.to_string());
        variables.insert("verification_score".to_string(), verification_score.to_string());
        variables.insert("market_analysis".to_string(), market_analysis.to_string());

        // Send email notification
        let email_id = self.send_notification(
            landlord.id,
            "property_live_email",
            variables.clone(),
            Some(landlord.email.clone()),
        ).await?;
        notification_ids.push(email_id);

        // Send SMS notification
        if let Some(phone) = &landlord.verification_number {
            let sms_id = self.send_notification(
                landlord.id,
                "verification_approved_sms",
                variables.clone(),
                Some(phone.clone()),
            ).await?;
            notification_ids.push(sms_id);
        }

        println!("🎉 Property live notifications sent to landlord {}", landlord.username);
        Ok(notification_ids)
    }

    /// Send notification when property is rejected
    pub async fn notify_property_rejected(
        &mut self,
        landlord: &User,
        property: &Property,
        rejection_stage: &str,
        rejection_reason: &str,
        rejection_details: &str,
    ) -> Result<Vec<Uuid>, NotificationError> {
        let mut variables = HashMap::new();
        variables.insert("landlord_name".to_string(), landlord.name.clone());
        variables.insert("property_title".to_string(), property.title.clone());
        variables.insert("property_address".to_string(), property.address.clone());
        variables.insert("property_reference".to_string(), 
            format!("VN-{}", property.id.to_string().chars().take(8).collect::<String>()));
        variables.insert("rejection_stage".to_string(), rejection_stage.to_string());
        variables.insert("rejection_reason".to_string(), rejection_reason.to_string());
        variables.insert("rejection_details".to_string(), rejection_details.to_string());

        let notification_id = self.send_notification(
            landlord.id,
            "property_rejected_email",
            variables,
            Some(landlord.email.clone()),
        ).await?;

        println!("❌ Property rejection notification sent to landlord {}", landlord.username);
        Ok(vec![notification_id])
    }

    /// Core notification sending method
    async fn send_notification(
        &mut self,
        user_id: Uuid,
        template_id: &str,
        variables: HashMap<String, String>,
        contact_info: Option<String>,
    ) -> Result<Uuid, NotificationError> {
        // Get template
        let template = self.templates.get(template_id)
            .ok_or_else(|| NotificationError::TemplateNotFound(template_id.to_string()))?
            .clone();

        // Check rate limits
        self.check_rate_limit(&template.channel, &user_id.to_string())?;

        // Render template
        let subject = self.render_template(&template.subject_template, &variables);
        let body = self.render_template(&template.body_template, &variables);

        // Create notification record
        let notification = Notification {
            id: Uuid::new_v4(),
            user_id,
            notification_type: template.notification_type.clone(),
            channel: template.channel.clone(),
            priority: template.priority.clone(),
            subject,
            body,
            data: variables,
            scheduled_at: Utc::now(),
            sent_at: None,
            read_at: None,
            status: NotificationStatus::Pending,
            retry_count: 0,
            max_retries: 3,
        };

        // Send through appropriate channel
        let notification_id = notification.id;
        match template.channel {
            NotificationChannel::Email => {
                if let Some(email) = contact_info {
                    self.send_email(&notification, &email).await?;
                }
            },
            NotificationChannel::SMS => {
                if let Some(phone) = contact_info {
                    self.send_sms(&notification, &phone).await?;
                }
            },
            NotificationChannel::WhatsApp => {
                if let Some(phone) = contact_info {
                    self.send_whatsapp(&notification, &phone).await?;
                }
            },
            _ => {
                return Err(NotificationError::InvalidChannel(
                    format!("Channel {:?} not implemented", template.channel)
                ));
            }
        }

        // Record rate limit
        self.record_rate_limit(&template.channel, &user_id.to_string());

        Ok(notification_id)
    }

    /// Render template with variables
    fn render_template(&self, template: &str, variables: &HashMap<String, String>) -> String {
        let mut result = template.to_string();
        for (key, value) in variables {
            result = result.replace(&format!("{{{}}}", key), value);
        }
        result
    }

    /// Check rate limits
    fn check_rate_limit(&self, channel: &NotificationChannel, user_key: &str) -> Result<(), NotificationError> {
        let rate_key = format!("{:?}_{}", channel, user_key);
        
        if let Some(timestamps) = self.rate_limiter.get(&rate_key) {
            let recent_count = timestamps.iter()
                .filter(|&&ts| ts > Utc::now() - Duration::hours(1))
                .count();

            let limit = match channel {
                NotificationChannel::Email => 20, // 20 emails per hour
                NotificationChannel::SMS => 10,   // 10 SMS per hour
                NotificationChannel::WhatsApp => 50, // 50 WhatsApp per hour
                _ => 100,
            };

            if recent_count >= limit {
                return Err(NotificationError::RateLimitExceeded(
                    format!("Rate limit exceeded for {:?}: {} >= {}", channel, recent_count, limit)
                ));
            }
        }

        Ok(())
    }

    /// Record rate limit timestamp
    fn record_rate_limit(&mut self, channel: &NotificationChannel, user_key: &str) {
        let rate_key = format!("{:?}_{}", channel, user_key);
        self.rate_limiter.entry(rate_key)
            .or_insert_with(Vec::new)
            .push(Utc::now());
    }

    /// Send email notification
    async fn send_email(&self, notification: &Notification, email: &str) -> Result<(), NotificationError> {
        // Using a simple email service (would integrate with SendGrid, AWS SES, etc.)
        println!("📧 Sending email to: {}", email);
        println!("📧 Subject: {}", notification.subject);
        println!("📧 Body preview: {}", &notification.body[..notification.body.len().min(100)]);

        // In production, implement actual email sending
        // Example with reqwest to an email service API:
        /*
        let email_payload = serde_json::json!({
            "to": email,
            "from": self.email_config.from_address,
            "subject": notification.subject,
            "html": notification.body,
            "priority": format!("{:?}", notification.priority)
        });

        let response = self.client
            .post("https://api.emailservice.com/send")
            .header("Authorization", format!("Bearer {}", self.email_config.api_key))
            .json(&email_payload)
            .send()
            .await
            .map_err(|e| NotificationError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(NotificationError::ExternalServiceError(
                format!("Email service returned: {}", response.status())
            ));
        }
        */

        Ok(())
    }

    /// Send SMS notification
    async fn send_sms(&self, notification: &Notification, phone: &str) -> Result<(), NotificationError> {
        println!("📱 Sending SMS to: {}", phone);
        println!("📱 Message: {}", notification.body);

        // Example implementation for Nigerian SMS provider (Termii)
        if self.sms_config.provider == "termii" {
            let sms_payload = serde_json::json!({
                "to": phone,
                "from": self.sms_config.sender_id,
                "sms": notification.body,
                "type": "plain",
                "api_key": self.sms_config.api_key,
                "channel": "generic"
            });

            // In production, send actual SMS
            /*
            let response = self.client
                .post("https://api.ng.termii.com/api/sms/send")
                .json(&sms_payload)
                .send()
                .await
                .map_err(|e| NotificationError::NetworkError(e.to_string()))?;

            if !response.status().is_success() {
                return Err(NotificationError::ExternalServiceError(
                    format!("SMS service returned: {}", response.status())
                ));
            }
            */
        }

        Ok(())
    }

    /// Send WhatsApp notification
    async fn send_whatsapp(&self, notification: &Notification, phone: &str) -> Result<(), NotificationError> {
        if let Some(whatsapp_config) = &self.whatsapp_config {
            println!("💬 Sending WhatsApp to: {}", phone);
            println!("💬 Message: {}", notification.body);

            // WhatsApp Business API implementation
            /*
            let whatsapp_payload = serde_json::json!({
                "messaging_product": "whatsapp",
                "to": phone,
                "text": {
                    "body": notification.body
                }
            });

            let response = self.client
                .post(&format!("https://graph.facebook.com/v18.0/{}/messages", 
                    whatsapp_config.phone_number_id))
                .header("Authorization", format!("Bearer {}", whatsapp_config.access_token))
                .json(&whatsapp_payload)
                .send()
                .await
                .map_err(|e| NotificationError::NetworkError(e.to_string()))?;

            if !response.status().is_success() {
                return Err(NotificationError::ExternalServiceError(
                    format!("WhatsApp service returned: {}", response.status())
                ));
            }
            */
        }

        Ok(())
    }

    /// Clean up old rate limit records (should be called periodically)
    pub fn cleanup_rate_limits(&mut self) {
        let cutoff_time = Utc::now() - Duration::hours(2);
        
        for timestamps in self.rate_limiter.values_mut() {
            timestamps.retain(|&ts| ts > cutoff_time);
        }
        
        self.rate_limiter.retain(|_, timestamps| !timestamps.is_empty());
    }

    /// Get notification statistics
    pub fn get_notification_stats(&self) -> HashMap<String, u32> {
        let mut stats = HashMap::new();
        
        for (key, timestamps) in &self.rate_limiter {
            let recent_count = timestamps.iter()
                .filter(|&&ts| ts > Utc::now() - Duration::hours(24))
                .count() as u32;
            
            stats.insert(key.clone(), recent_count);
        }
        
        stats
    }
}