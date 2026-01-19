//! Upbit JWT Authentication
//!
//! Upbit API 인증을 위한 JWT 토큰 생성

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use uuid::Uuid;

/// JWT 클레임 (Upbit API 요구사항)
#[derive(Debug, Serialize)]
struct UpbitClaims {
    access_key: String,
    nonce: String,
    timestamp: i64,
}

/// Upbit API 인증용 JWT 토큰을 생성합니다.
///
/// # Arguments
/// * `access_key` - Upbit API Access Key
/// * `secret_key` - Upbit API Secret Key
///
/// # Returns
/// * `Ok(String)` - 생성된 JWT 토큰
/// * `Err(String)` - 토큰 생성 실패 시 에러 메시지
pub fn generate_jwt_token(access_key: &str, secret_key: &str) -> Result<String, String> {
    let claims = UpbitClaims {
        access_key: access_key.to_string(),
        nonce: Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
    };

    let header = Header::new(Algorithm::HS256);
    let encoding_key =
        EncodingKey::from_secret(secret_key.as_bytes());

    encode(&header, &claims, &encoding_key)
        .map_err(|e| format!("JWT 토큰 생성 실패: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_token_format() {
        // JWT는 header.payload.signature 형식 (3파트)
        let token = generate_jwt_token("test_access_key", "test_secret_key").unwrap();
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT 토큰은 3개의 파트로 구성되어야 합니다");
    }

    #[test]
    fn test_jwt_token_not_empty() {
        let token = generate_jwt_token("test_key", "test_secret").unwrap();
        assert!(!token.is_empty(), "JWT 토큰이 비어있으면 안됩니다");
    }

    #[test]
    fn test_different_nonce_each_call() {
        // 매번 다른 nonce가 생성되어야 함
        let token1 = generate_jwt_token("key", "secret").unwrap();
        let token2 = generate_jwt_token("key", "secret").unwrap();
        assert_ne!(token1, token2, "각 호출마다 다른 토큰이 생성되어야 합니다");
    }
}
