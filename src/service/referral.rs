use rand::{distr::Alphanumeric, Rng};

pub fn generate_referral_code() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect::<String>()
        .to_uppercase()
}

pub fn generate_referral_link(base_url: &str, code: &str) -> String {
    format!("{}/register?ref={}", base_url, code)
}