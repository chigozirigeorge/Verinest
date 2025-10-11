// config.rs (Updated)
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub app_url: String,
    pub jwt_secret: String,
    pub jwt_maxage: i64,
    pub port: u16,
    // Add payment provider configurations
    pub paystack_secret_key: String,
    pub flutterwave_secret_key: String,
    pub active_payment_provider: String,
    // Add email service configurations
    pub smtp_host: String,
    pub smtp_username: String,
    pub smtp_password: String,
    // Add other service configurations as needed
}

impl Config {
    pub fn init() -> Config {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let jwt_secret = std::env::var("JWT_SECRET_KEY").expect("JWT_SECRET_KEY must be set");
        let jwt_maxage = std::env::var("JWT_MAXAGE").expect("JWT_MAXAGE must be set");
        let app_url = std::env::var("APP_URL").expect("APP_URL must be set");
        
        // Payment provider configurations (with defaults)
        let paystack_secret_key = std::env::var("PAYSTACK_SECRET_KEY")
            .unwrap_or_else(|_| "test_secret_key".to_string());
        let flutterwave_secret_key = std::env::var("FLUTTERWAVE_SECRET_KEY")
            .unwrap_or_else(|_| "test_secret_key".to_string());
        let active_payment_provider = std::env::var("ACTIVE_PAYMENT_PROVIDER")
            .unwrap_or_else(|_| "paystack".to_string());
            
        // Email service configurations (with defaults)
        let smtp_host = std::env::var("SMTP_HOST")
            .unwrap_or_else(|_| "localhost".to_string());
        let smtp_username = std::env::var("SMTP_USERNAME")
            .unwrap_or_else(|_| "".to_string());
        let smtp_password = std::env::var("SMTP_PASSWORD")
            .unwrap_or_else(|_| "".to_string());

        Config {
            database_url,
            app_url,
            jwt_secret,
            jwt_maxage: jwt_maxage.parse::<i64>().unwrap(),
            port: 8000,
            paystack_secret_key,
            flutterwave_secret_key,
            active_payment_provider,
            smtp_host,
            smtp_username,
            smtp_password,
        }
    }
}