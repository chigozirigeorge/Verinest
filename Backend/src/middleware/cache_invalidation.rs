// Cache invalidation pairing system
use redis::aio::ConnectionManager;
use crate::db::cache::CacheHelper;
use std::sync::Arc;

/// Cache invalidation groups - endpoints that should invalidate each other's cache
#[derive(Debug, Clone)]
pub struct CacheGroup {
    pub name: String,
    pub patterns: Vec<String>,
    pub get_endpoints: Vec<String>,
    pub mutate_endpoints: Vec<String>,
}

impl CacheGroup {
    pub fn new(name: &str, patterns: Vec<&str>, get_endpoints: Vec<&str>, mutate_endpoints: Vec<&str>) -> Self {
        Self {
            name: name.to_string(),
            patterns: patterns.iter().map(|s| s.to_string()).collect(),
            get_endpoints: get_endpoints.iter().map(|s| s.to_string()).collect(),
            mutate_endpoints: mutate_endpoints.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// Cache invalidation configuration
pub struct CacheInvalidationConfig {
    pub groups: Vec<CacheGroup>,
}

impl CacheInvalidationConfig {
    pub fn new() -> Self {
        let groups = vec![
            // WALLET GROUP - Most critical for your deposit issue
            CacheGroup::new(
                "wallet",
                vec![
                    "cache:*wallet*",
                    "cache:*users/*/wallet*",
                ],
                vec![
                    "GET /wallet",
                    "GET /wallet/",
                    "GET /wallet/summary", 
                    "GET /wallet/transactions",
                    "GET /wallet/transaction/*",
                    "GET /wallet/bank-accounts",
                ],
                vec![
                    "POST /wallet/create",
                    "POST /wallet/deposit",
                    "POST /wallet/withdraw", 
                    "POST /wallet/transfer",
                    "POST /wallet/bank-accounts",
                    "POST /wallet/bank-accounts/*/verify",
                    "PUT /wallet/bank-accounts/*/primary",
                    "POST /wallet/deposit/verify", 
                    "GET /wallet/deposit/verify", 
                ]
            ),

            // USER PROFILE GROUP
            CacheGroup::new(
                "user_profile",
                vec![
                    "cache:*api/users*",
                    "cache:*api/auth*",
                ],
                vec![
                    "GET /api/users/profile",
                    "GET /api/users/*/profile", 
                    "GET /api/users/subscription/premium",
                    "GET /api/users/subscription/role-change-stats",
                    "GET /api/users/subscription/benefits",
                ],
                vec![
                    "PUT /api/users/profile",
                    "PUT /api/users/name",
                    "PUT /api/users/avatar", 
                    "PUT /api/users/password",
                    "PUT /api/users/role",
                    "PUT /api/users/role/upgrade",
                    "PUT /api/users/transaction-pin",
                    "POST /api/users/transaction-pin/verify",
                    "POST /api/users/subscription/premium",
                    "POST /api/users/subscription/premium/initiate",
                    "POST /api/auth/logout",
                ]
            ),

            // LABOUR GROUP
            CacheGroup::new(
                "labour",
                vec![
                    "cache:*labour*",
                ],
                vec![
                    "GET /labour/worker/profile",
                    "GET /labour/worker/portfolio",
                    "GET /labour/jobs",
                    "GET /labour/jobs/*",
                    "GET /labour/workers/search",
                    "GET /labour/workers/*",
                    "GET /labour/worker/dashboard",
                    "GET /labour/employer/dashboard",
                    "GET /labour/contracts",
                    "GET /labour/contracts/*",
                    "GET /labour/applications/*",
                ],
                vec![
                    "POST /labour/worker/profile",
                    "PUT /labour/worker/profile/availability",
                    "POST /labour/worker/portfolio",
                    "DELETE /labour/worker/portfolio/*",
                    "POST /labour/jobs",
                    "POST /labour/jobs/*/applications",
                    "PUT /labour/jobs/*/assign",
                    "POST /labour/jobs/*/contract",
                    "PUT /labour/jobs/*/complete",
                    "POST /labour/jobs/*/review",
                    "POST /labour/jobs/*/dispute",
                    "PUT /labour/disputes/*/resolve",
                    "PUT /labour/contracts/*/sign",
                    "PUT /labour/applications/*/review",
                    "POST /labour/jobs/*/escrow/release",
                ]
            ),

            // VENDOR GROUP
            CacheGroup::new(
                "vendor",
                vec![
                    "cache:*api/vendor*",
                    "cache:*api/services*",
                    "cache:*api/orders*",
                ],
                vec![
                    "GET /api/vendor/profile",
                    "GET /api/vendor/services",
                    "GET /api/vendor/services/*",
                    "GET /api/vendor/subscription/status",
                    "GET /api/vendor/inquiries",
                    "GET /api/vendor/analytics",
                    "GET /api/vendor/orders",
                    "GET /api/services",
                    "GET /api/services/recommended",
                    "GET /api/services/*",
                    "GET /api/orders/*",
                    "GET /api/orders/my-purchases",
                    "GET /api/services/*/reviews",
                ],
                vec![
                    "POST /api/vendor/profile",
                    "PUT /api/vendor/profile",
                    "POST /api/vendor/subscription/upgrade",
                    "POST /api/vendor/services",
                    "PUT /api/vendor/services/*",
                    "PUT /api/vendor/services/*/status",
                    "POST /api/services/*/inquiry",
                    "POST /api/services/*/purchase",
                    "POST /api/orders/*/complete",
                    "POST /api/orders/*/cancel",
                    "POST /api/vendor/orders/*/confirm",
                    "POST /api/orders/*/delivery/confirm",
                    "POST /api/admin/payments/verify",
                    "POST /api/orders/*/review",
                    "POST /api/vendor/orders/*/confirm",
                ]
            ),

            // CHAT GROUP
            CacheGroup::new(
                "chat",
                vec![
                    "cache:*chat*",
                ],
                vec![
                    "GET /chat/chats",
                    "GET /chat/chats/*",
                    "GET /chat/chats/*/messages",
                    "GET /chat/unread-count",
                ],
                vec![
                    "POST /chat/chats",
                    "POST /chat/chats/*/messages",
                    "PUT /chat/chats/*/read",
                    "POST /chat/chats/*/contract-proposal",
                    "PUT /chat/contract-proposals/*/respond",
                ]
            ),

            // NOTIFICATION GROUP
            CacheGroup::new(
                "notifications",
                vec![
                    "cache:*notifications*",
                ],
                vec![
                    "GET /notifications/",
                    "GET /notifications/unread-count",
                ],
                vec![
                    "POST /notifications/read",
                    "POST /notifications/read-all",
                    "PUT /notifications/*/read",
                    "DELETE /notifications/*",
                ]
            ),

            // SUPPORT GROUP
            CacheGroup::new(
                "support",
                vec![
                    "cache:*api/support*",
                ],
                vec![
                    "GET /api/support/tickets",
                    "GET /api/support/tickets/*",
                    "GET /api/support/tickets/*/messages",
                    "GET /api/support/my-tickets",
                ],
                vec![
                    "POST /api/support/tickets",
                    "PUT /api/support/tickets/*/status",
                    "PUT /api/support/tickets/*/assign",
                    "POST /api/support/tickets/*/messages",
                ]
            ),

            // VERIFICATION GROUP
            CacheGroup::new(
                "verification",
                vec![
                    "cache:*api/verification*",
                ],
                vec![
                    "GET /api/verification/documents",
                    "GET /api/verification/status",
                    "GET /api/verification/complete-status",
                    "GET /api/verification/admin/pending",
                ],
                vec![
                    "POST /api/verification/otp/send",
                    "POST /api/verification/otp/verify",
                    "POST /api/verification/nin",
                    "POST /api/verification/document",
                    "PUT /api/verification/admin/*/review",
                ]
            ),
        ];

        Self { groups }
    }

    /// Find which cache group a request belongs to
    pub fn find_group_for_request(&self, method: &str, path: &str) -> Option<&CacheGroup> {
        let normalized_path = self.normalize_path(path);
        let request_key = format!("{} {}", method, normalized_path);

        for group in &self.groups {
            // Check if it's a GET endpoint
            if method == "GET" {
                for get_endpoint in &group.get_endpoints {
                    if self.matches_endpoint(&request_key, get_endpoint) {
                        return Some(group);
                    }
                }
            }
            
            // Check if it's a mutation endpoint
            if method == "POST" || method == "PUT" || method == "DELETE" {
                for mutate_endpoint in &group.mutate_endpoints {
                    if self.matches_endpoint(&request_key, mutate_endpoint) {
                        return Some(group);
                    }
                }
            }
        }

        None
    }

    /// Get cache patterns to invalidate for a given request
    pub fn get_invalidation_patterns(&self, method: &str, path: &str) -> Vec<String> {
        if let Some(group) = self.find_group_for_request(method, path) {
            // Invalidate on mutations (POST/PUT/DELETE) AND specific GET endpoints
            if method == "POST" || method == "PUT" || method == "DELETE" {
                return group.patterns.clone();
            }
            
            // Special case: Allow certain GET endpoints to invalidate cache
            // (like deposit verification that updates wallet balance)
            if method == "GET" {
                // Check if this GET endpoint should invalidate cache
                for endpoint in &group.mutate_endpoints {
                    if self.matches_endpoint(&format!("{} {}", method, path), endpoint) {
                        tracing::info!("🗑️  GET endpoint {} matches invalidation pattern", path);
                        return group.patterns.clone();
                    }
                }
            }
        }
        vec![]
    }

    /// Normalize path by replacing UUIDs with wildcards
    fn normalize_path(&self, path: &str) -> String {
        let parts: Vec<&str> = path.split('/').collect();
        let mut normalized_parts = Vec::new();

        for part in parts {
            if part.len() == 36 && part.chars().nth(8) == Some('-') {
                // This looks like a UUID, replace with wildcard
                normalized_parts.push("*");
            } else if part.parse::<u64>().is_ok() {
                // This looks like a numeric ID, replace with wildcard
                normalized_parts.push("*");
            } else {
                normalized_parts.push(part);
            }
        }

        normalized_parts.join("/")
    }

    /// Check if request matches an endpoint pattern
    fn matches_endpoint(&self, request: &str, endpoint: &str) -> bool {
        // Simple pattern matching - can be enhanced with regex if needed
        if endpoint.contains('*') {
            // Convert to simple regex pattern
            let pattern = endpoint.replace('*', ".*");
            let regex = regex::Regex::new(&format!("^{}$", pattern)).unwrap_or(regex::Regex::new(r".*").unwrap());
            regex.is_match(request)
        } else {
            request == endpoint
        }
    }
}

/// Invalidate cache based on request method and path
pub async fn invalidate_cache_for_request(
    redis_client: &ConnectionManager,
    config: &CacheInvalidationConfig,
    method: &str,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let patterns = config.get_invalidation_patterns(method, path);
    
    if !patterns.is_empty() {
        tracing::info!("🗑️  Invalidating cache for {} {} - patterns: {:?}", method, path, patterns);
        
        for pattern in patterns {
            let _ = CacheHelper::delete_pattern(&Arc::new(redis_client.clone()), &pattern).await;
        }
    }
    
    Ok(())
}

/// Check if request should be cached
pub fn should_cache_request(method: &str, path: &str, config: &CacheInvalidationConfig) -> bool {
    // Only cache GET requests
    if method != "GET" {
        return false;
    }

    // Check if it matches any GET endpoint in our groups
    config.find_group_for_request(method, path).is_some()
}
