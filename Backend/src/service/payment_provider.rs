// service/payment_provider.rs
use std::env;
use serde::{Deserialize, Serialize};
use reqwest;

use crate::{
    models::walletmodels::PaymentMethod,
    config::Config,
    AppState,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentInitResponse {
    pub payment_url: String,
    pub access_code: String,
    pub reference: String,
    pub amount: f64,
    pub fee: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentVerification {
    pub status: String,
    pub amount: i64,
    pub gateway_reference: String,
    pub paid_at: String,
    pub channel: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountResolution {
    pub account_number: String,
    pub account_name: String,
    pub bank_code: String,
    pub bank_name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransferInitiation {
    pub reference: String,
    pub transfer_code: String,
    pub status: String,
}

pub struct PaymentProviderService {
    paystack_secret_key: String,
    flutterwave_secret_key: String,
    active_provider: String, // "paystack" or "flutterwave"
}

impl PaymentProviderService {
    pub fn new(config: &Config) -> Self {
        Self {
            paystack_secret_key: config.paystack_secret_key.clone(),
            flutterwave_secret_key: config.flutterwave_secret_key.clone(),
            active_provider: config.active_payment_provider.clone(),
        }
    }

    // Initialize deposit payment
    pub async fn initialize_payment(
        &self,
        email: String,
        amount: f64,
        reference: String,
        payment_method: PaymentMethod,
        metadata: Option<serde_json::Value>,
    ) -> Result<PaymentInitResponse, Box<dyn std::error::Error>> {
        match self.active_provider.as_str() {
            "paystack" => self.paystack_initialize_payment(email, amount, reference, metadata).await,
            "flutterwave" => self.flutterwave_initialize_payment(email, amount, reference, metadata).await,
            _ => Err("Invalid payment provider".into()),
        }
    }

    // Paystack: Initialize payment
    async fn paystack_initialize_payment(
        &self,
        email: String,
        amount: f64,
        reference: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<PaymentInitResponse, Box<dyn std::error::Error>> {
        let amount_kobo = (amount * 100.0) as i64;
        
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "email": email,
            "amount": amount_kobo,
            "reference": reference,
            "currency": "NGN",
            "metadata": metadata.unwrap_or(serde_json::json!({})),
            "channels": ["card", "bank", "ussd", "qr", "mobile_money", "bank_transfer"]
        });

        let response = client
            .post("https://api.paystack.co/transaction/initialize")
            .header("Authorization", format!("Bearer {}", self.paystack_secret_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let response_body: serde_json::Value = response.json().await?;
        
        if response_body["status"].as_bool().unwrap_or(false) {
            let data = &response_body["data"];
            Ok(PaymentInitResponse {
                payment_url: data["authorization_url"].as_str().unwrap_or("").to_string(),
                access_code: data["access_code"].as_str().unwrap_or("").to_string(),
                reference: data["reference"].as_str().unwrap_or("").to_string(),
                amount,
                fee: 0.0, // Paystack fees are deducted from merchant
            })
        } else {
            Err(response_body["message"].as_str().unwrap_or("Payment initialization failed").into())
        }
    }

    // Flutterwave: Initialize payment
    async fn flutterwave_initialize_payment(
        &self,
        email: String,
        amount: f64,
        reference: String,
        metadata: Option<serde_json::Value>,
    ) -> Result<PaymentInitResponse, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let payload = serde_json::json!({
            "tx_ref": reference,
            "amount": amount,
            "currency": "NGN",
            "redirect_url": env::var("FLUTTERWAVE_REDIRECT_URL").unwrap_or_default(),
            "payment_options": "card,banktransfer,ussd,account",
            "customer": {
                "email": email,
            },
            "customizations": {
                "title": "Verinest Wallet Deposit",
                "description": "Fund your Verinest wallet",
                "logo": env::var("APP_LOGO_URL").unwrap_or_default(),
            },
            "meta": metadata.unwrap_or(serde_json::json!({}))
        });

        let response = client
            .post("https://api.flutterwave.com/v3/payments")
            .header("Authorization", format!("Bearer {}", self.flutterwave_secret_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let response_body: serde_json::Value = response.json().await?;
        
        if response_body["status"].as_str() == Some("success") {
            let data = &response_body["data"];
            Ok(PaymentInitResponse {
                payment_url: data["link"].as_str().unwrap_or("").to_string(),
                access_code: "".to_string(),
                reference,
                amount,
                fee: 0.0,
            })
        } else {
            Err(response_body["message"].as_str().unwrap_or("Payment initialization failed").into())
        }
    }

    // Verify payment
    pub async fn verify_payment(
        &self,
        reference: &str,
    ) -> Result<PaymentVerification, Box<dyn std::error::Error>> {
        match self.active_provider.as_str() {
            "paystack" => self.paystack_verify_payment(reference).await,
            "flutterwave" => self.flutterwave_verify_payment(reference).await,
            _ => Err("Invalid payment provider".into()),
        }
    }

    // Paystack: Verify payment
    async fn paystack_verify_payment(
        &self,
        reference: &str,
    ) -> Result<PaymentVerification, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let url = format!("https://api.paystack.co/transaction/verify/{}", reference);

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.paystack_secret_key))
            .send()
            .await?;

        let response_body: serde_json::Value = response.json().await?;
        
        if response_body["status"].as_bool().unwrap_or(false) {
            let data = &response_body["data"];
            
            if data["status"].as_str() == Some("success") {
                Ok(PaymentVerification {
                    status: "success".to_string(),
                    amount: data["amount"].as_i64().unwrap_or(0),
                    gateway_reference: data["reference"].as_str().unwrap_or("").to_string(),
                    paid_at: data["paid_at"].as_str().unwrap_or("").to_string(),
                    channel: data["channel"].as_str().unwrap_or("").to_string(),
                    metadata: data.get("metadata").cloned(),
                })
            } else {
                Err("Payment not successful".into())
            }
        } else {
            Err(response_body["message"].as_str().unwrap_or("Verification failed").into())
        }
    }

    // Flutterwave: Verify payment
    async fn flutterwave_verify_payment(
        &self,
        reference: &str,
    ) -> Result<PaymentVerification, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let url = format!("https://api.flutterwave.com/v3/transactions/{}/verify", reference);

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.flutterwave_secret_key))
            .send()
            .await?;

        let response_body: serde_json::Value = response.json().await?;
        
        if response_body["status"].as_str() == Some("success") {
            let data = &response_body["data"];
            
            if data["status"].as_str() == Some("successful") {
                Ok(PaymentVerification {
                    status: "success".to_string(),
                    amount: data["amount"].as_f64().unwrap_or(0.0) as i64 * 100, // Convert to kobo
                    gateway_reference: data["flw_ref"].as_str().unwrap_or("").to_string(),
                    paid_at: data["created_at"].as_str().unwrap_or("").to_string(),
                    channel: data["payment_type"].as_str().unwrap_or("").to_string(),
                    metadata: data.get("meta").cloned(),
                })
            } else {
                Err("Payment not successful".into())
            }
        } else {
            Err("Verification failed".into())
        }
    }

    // Resolve account number (Paystack)
    pub async fn resolve_account_number(
        &self,
        account_number: &str,
        bank_code: &str,
    ) -> Result<AccountResolution, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let url = format!(
            "https://api.paystack.co/bank/resolve?account_number={}&bank_code={}",
            account_number, bank_code
        );

        let response = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.paystack_secret_key))
            .send()
            .await?;

        let response_body: serde_json::Value = response.json().await?;
        
        if response_body["status"].as_bool().unwrap_or(false) {
            let data = &response_body["data"];
            Ok(AccountResolution {
                account_number: account_number.to_string(),
                account_name: data["account_name"].as_str().unwrap_or("").to_string(),
                bank_code: bank_code.to_string(),
                bank_name: self.get_bank_name(bank_code).await?,
            })
        } else {
            Err(response_body["message"].as_str().unwrap_or("Account resolution failed").into())
        }
    }

    // Initiate transfer (withdrawal)
    pub async fn initiate_transfer(
        &self,
        account_number: String,
        bank_code: String,
        amount: f64,
        reference: String,
        narration: String,
    ) -> Result<TransferInitiation, Box<dyn std::error::Error>> {
        match self.active_provider.as_str() {
            "paystack" => self.paystack_initiate_transfer(
                account_number,
                bank_code,
                amount,
                reference,
                narration,
            ).await,
            "flutterwave" => self.flutterwave_initiate_transfer(
                account_number,
                bank_code,
                amount,
                reference,
                narration,
            ).await,
            _ => Err("Invalid payment provider".into()),
        }
    }

    // Paystack: Create transfer recipient and initiate transfer
    async fn paystack_initiate_transfer(
        &self,
        account_number: String,
        bank_code: String,
        amount: f64,
        reference: String,
        narration: String,
    ) -> Result<TransferInitiation, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        
        // First, create transfer recipient
        let recipient_payload = serde_json::json!({
            "type": "nuban",
            "name": "Recipient",
            "account_number": account_number,
            "bank_code": bank_code,
            "currency": "NGN"
        });

        let recipient_response = client
            .post("https://api.paystack.co/transferrecipient")
            .header("Authorization", format!("Bearer {}", self.paystack_secret_key))
            .header("Content-Type", "application/json")
            .json(&recipient_payload)
            .send()
            .await?;

        let recipient_body: serde_json::Value = recipient_response.json().await?;
        
        if !recipient_body["status"].as_bool().unwrap_or(false) {
            return Err("Failed to create transfer recipient".into());
        }

        let recipient_code = recipient_body["data"]["recipient_code"]
            .as_str()
            .ok_or("Missing recipient code")?;

        // Now initiate transfer
        let amount_kobo = (amount * 100.0) as i64;
        let transfer_payload = serde_json::json!({
            "source": "balance",
            "amount": amount_kobo,
            "reference": reference,
            "recipient": recipient_code,
            "reason": narration
        });

        let transfer_response = client
            .post("https://api.paystack.co/transfer")
            .header("Authorization", format!("Bearer {}", self.paystack_secret_key))
            .header("Content-Type", "application/json")
            .json(&transfer_payload)
            .send()
            .await?;

        let transfer_body: serde_json::Value = transfer_response.json().await?;
        
        if transfer_body["status"].as_bool().unwrap_or(false) {
            let data = &transfer_body["data"];
            Ok(TransferInitiation {
                reference: data["reference"].as_str().unwrap_or("").to_string(),
                transfer_code: data["transfer_code"].as_str().unwrap_or("").to_string(),
                status: data["status"].as_str().unwrap_or("pending").to_string(),
            })
        } else {
            Err(transfer_body["message"].as_str().unwrap_or("Transfer failed").into())
        }
    }

    // Flutterwave: Initiate transfer
    async fn flutterwave_initiate_transfer(
        &self,
        account_number: String,
        bank_code: String,
        amount: f64,
        reference: String,
        narration: String,
    ) -> Result<TransferInitiation, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        
        let payload = serde_json::json!({
            "account_bank": bank_code,
            "account_number": account_number,
            "amount": amount,
            "narration": narration,
            "currency": "NGN",
            "reference": reference,
            "callback_url": env::var("FLUTTERWAVE_CALLBACK_URL").unwrap_or_default(),
            "debit_currency": "NGN"
        });

        let response = client
            .post("https://api.flutterwave.com/v3/transfers")
            .header("Authorization", format!("Bearer {}", self.flutterwave_secret_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        let response_body: serde_json::Value = response.json().await?;
        
        if response_body["status"].as_str() == Some("success") {
            let data = &response_body["data"];
            Ok(TransferInitiation {
                reference: data["reference"].as_str().unwrap_or("").to_string(),
                transfer_code: data["id"].as_i64().unwrap_or(0).to_string(),
                status: data["status"].as_str().unwrap_or("pending").to_string(),
            })
        } else {
            Err(response_body["message"].as_str().unwrap_or("Transfer failed").into())
        }
    }

    // Get bank name from bank code
    async fn get_bank_name(&self, bank_code: &str) -> Result<String, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.paystack.co/bank")
            .header("Authorization", format!("Bearer {}", self.paystack_secret_key))
            .send()
            .await?;

        let response_body: serde_json::Value = response.json().await?;
        
        if let Some(banks) = response_body["data"].as_array() {
            for bank in banks {
                if bank["code"].as_str() == Some(bank_code) {
                    return Ok(bank["name"].as_str().unwrap_or("Unknown Bank").to_string());
                }
            }
        }
        
        Ok("Unknown Bank".to_string())
    }

    // Get list of Nigerian banks
    pub async fn get_banks(&self) -> Result<Vec<serde_json::Value>, Box<dyn std::error::Error>> {
        let client = reqwest::Client::new();
        let response = client
            .get("https://api.paystack.co/bank")
            .header("Authorization", format!("Bearer {}", self.paystack_secret_key))
            .send()
            .await?;

        let response_body: serde_json::Value = response.json().await?;
        
        if response_body["status"].as_bool().unwrap_or(false) {
            Ok(response_body["data"]
                .as_array()
                .unwrap_or(&vec![])
                .clone())
        } else {
            Err("Failed to fetch banks".into())
        }
    }
}