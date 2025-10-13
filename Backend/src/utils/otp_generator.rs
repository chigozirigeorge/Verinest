// utils/otp_generator.rs
use rand::Rng;

pub fn generate_otp() -> String {
    let mut rng = rand::rng();
    format!("{:06}", rng.random_range(100000..999999))
}

pub fn generate_secure_otp() -> String {
    use rand::distr::Alphanumeric;
    use rand::{rng, Rng};
    
    let mut rng = rng();
    (0..8)
        .map(|_| rng.sample(Alphanumeric) as char)
        .collect()
}