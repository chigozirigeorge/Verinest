use bigdecimal::BigDecimal;
use num_traits::ToPrimitive;

pub trait BigDecimalHelpers {
    fn to_i64_or_zero(&self) -> i64;
}

impl BigDecimalHelpers for BigDecimal {
    fn to_i64_or_zero(&self) -> i64 {
        self.to_i64().unwrap_or(0)
    }
}

impl BigDecimalHelpers for Option<BigDecimal> {
    fn to_i64_or_zero(&self) -> i64 {
        self.as_ref()
            .map(|bd| bd.to_i64().unwrap_or(0))
            .unwrap_or(0)
    }
}