pub mod jwt;
pub mod middleware;
pub mod oidc;
pub mod password;

pub use jwt::{issue_token, issue_token_pair, validate_refresh_token, Claims, TokenPair};
pub use middleware::{require_auth, AuthUser};
pub use oidc::{discover_oidc, exchange_code, extract_role, DiscoverParams, OidcConfig};
pub use password::{hash_password, verify_password};
