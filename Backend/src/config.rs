// config.rs (Updated with Redis)
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub app_url: String,
    pub jwt_secret: String,
    pub jwt_maxage: i64,
    pub port: u16,
    // Redis configuration
    pub redis_url: Option<String>,
    pub redis_enabled: bool,
    // Payment provider configurations
    pub paystack_secret_key: String,
    pub flutterwave_secret_key: String,
    pub active_payment_provider: String,
    // Email service configurations
    pub smtp_host: String,
    pub smtp_username: String,
    pub smtp_password: String,
    pub smtp_port: u16,
    pub resend_api_key: String,
    pub from_email: String,
    pub email_rate_limit: usize,
    pub email_rate_window_minutes: i64,
}

impl Config {
    pub fn init() -> Config {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let jwt_secret = std::env::var("JWT_SECRET_KEY").expect("JWT_SECRET_KEY must be set");
        let jwt_maxage = std::env::var("JWT_MAXAGE").expect("JWT_MAXAGE must be set");
        let app_url = std::env::var("APP_URL").expect("APP_URL must be set");
        
        // Redis configuration (optional)
        let redis_url = std::env::var("REDIS_URL").ok();
        let redis_enabled = redis_url.is_some();
        
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
        let smtp_port: u16 = std::env::var("SMTP_PORT")
            .unwrap_or_else(|_| "587".to_string())
            .parse()
            .unwrap_or(587);
        let resend_api_key = std::env::var("RESEND_API_KEY")
            .unwrap_or_else(|_| "".to_string());
        let from_email = std::env::var("FROM_EMAIL")
            .unwrap_or_else(|_| "Verinest <noreply@verinest.xyz>".to_string());
        let email_rate_limit: usize = std::env::var("EMAIL_RATE_LIMIT")
            .unwrap_or_else(|_| "10".to_string())
            .parse()
            .unwrap_or(10);
        let email_rate_window_minutes: i64 = std::env::var("EMAIL_RATE_WINDOW_MINUTES")
            .unwrap_or_else(|_| "1".to_string())
            .parse()
            .unwrap_or(1);

        if redis_enabled {
            println!("🚀 Redis caching is ENABLED");
        } else {
            println!("⚠️  Redis caching is DISABLED (set REDIS_URL to enable)");
        }

        Config {
            database_url,
            app_url,
            jwt_secret,
            jwt_maxage: jwt_maxage.parse::<i64>().unwrap(),
            port: 8000,
            redis_url,
            redis_enabled,
            paystack_secret_key,
            flutterwave_secret_key,
            active_payment_provider,
            smtp_host,
            smtp_username,
            smtp_password,
            smtp_port,
            resend_api_key,
            from_email,
            email_rate_limit,
            email_rate_window_minutes,
        }
    }
}