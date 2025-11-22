// Database query timeout protection
use sqlx::postgres::PgPool;
use std::time::Duration;
use tokio::time::timeout;

pub struct QueryTimeout;

impl QueryTimeout {
    /// Execute a query with a timeout to prevent slow queries from blocking
    pub async fn execute_with_timeout<F, T>(
        pool: &PgPool,
        query_fn: F,
        timeout_duration: Duration,
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
    where
        F: std::future::Future<Output = Result<T, sqlx::Error>>,
    {
        match timeout(timeout_duration, query_fn).await {
            Ok(result) => result.map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
            Err(_) => Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                format!("Query timed out after {:?}", timeout_duration),
            )) as Box<dyn std::error::Error + Send + Sync>),
        }
    }

    /// Default timeout for most queries (5 seconds)
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);
    
    /// Longer timeout for complex aggregation queries (30 seconds)
    pub const AGGREGATION_TIMEOUT: Duration = Duration::from_secs(30);
    
    /// Short timeout for simple lookups (2 seconds)
    pub const LOOKUP_TIMEOUT: Duration = Duration::from_secs(2);
}
