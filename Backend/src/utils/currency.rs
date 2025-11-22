/// Currency utility functions for handling Naira conversions
/// 
/// All monetary values in the database are stored in kobo (1 Naira = 100 kobo)
/// to avoid floating-point precision issues.

/// Convert Naira to kobo (multiply by 100)
pub fn naira_to_kobo(naira: f64) -> i64 {
    (naira * 100.0).round() as i64
}

/// Convert kobo to Naira (divide by 100)
pub fn kobo_to_naira(kobo: i64) -> f64 {
    kobo as f64 / 100.0
}

/// Format kobo as Naira string with 2 decimal places
pub fn format_kobo_as_naira(kobo: i64) -> String {
    format!("₦{:.2}", kobo_to_naira(kobo))
}

/// Validate and parse amount string to kobo
pub fn parse_amount_to_kobo(amount_str: &str) -> Result<i64, String> {
    amount_str
        .parse::<f64>()
        .map_err(|_| "Invalid amount format".to_string())
        .and_then(|amount| {
            if amount < 0.0 {
                Err("Amount cannot be negative".to_string())
            } else {
                Ok(naira_to_kobo(amount))
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_naira_to_kobo() {
        assert_eq!(naira_to_kobo(100.0), 10000);
        assert_eq!(naira_to_kobo(0.50), 50);
        assert_eq!(naira_to_kobo(123.45), 12345);
    }

    #[test]
    fn test_kobo_to_naira() {
        assert_eq!(kobo_to_naira(10000), 100.0);
        assert_eq!(kobo_to_naira(50), 0.50);
        assert_eq!(kobo_to_naira(12345), 123.45);
    }

    #[test]
    fn test_format_kobo_as_naira() {
        assert_eq!(format_kobo_as_naira(10000), "₦100.00");
        assert_eq!(format_kobo_as_naira(50), "₦0.50");
        assert_eq!(format_kobo_as_naira(12345), "₦123.45");
    }

    #[test]
    fn test_parse_amount_to_kobo() {
        assert_eq!(parse_amount_to_kobo("100.00"), Ok(10000));
        assert_eq!(parse_amount_to_kobo("0.50"), Ok(50));
        assert_eq!(parse_amount_to_kobo("-100"), Err("Amount cannot be negative".to_string()));
        assert_eq!(parse_amount_to_kobo("abc"), Err("Invalid amount format".to_string()));
    }
}
