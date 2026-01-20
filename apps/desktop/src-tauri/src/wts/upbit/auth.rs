//! Upbit JWT Authentication
//!
//! Upbit API 인증을 위한 JWT 토큰 생성

use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use sha2::{Digest, Sha512};
use uuid::Uuid;

/// JWT 클레임 (Upbit API 요구사항 - GET 요청용)
#[derive(Debug, Serialize)]
struct UpbitClaims {
    access_key: String,
    nonce: String,
    timestamp: i64,
}

/// JWT 클레임 (쿼리 해시 포함 - POST 요청용)
#[derive(Debug, Serialize)]
struct UpbitClaimsWithQuery {
    access_key: String,
    nonce: String,
    timestamp: i64,
    query_hash: String,
    query_hash_alg: String,
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

/// Upbit API 인증용 JWT 토큰을 생성합니다 (쿼리 해시 포함, POST 요청용).
///
/// # Arguments
/// * `access_key` - Upbit API Access Key
/// * `secret_key` - Upbit API Secret Key
/// * `query` - 요청 본문 (JSON 문자열)
///
/// # Returns
/// * `Ok(String)` - 생성된 JWT 토큰
/// * `Err(String)` - 토큰 생성 실패 시 에러 메시지
pub fn generate_jwt_token_with_query(
    access_key: &str,
    secret_key: &str,
    query: &str,
) -> Result<String, String> {
    // SHA-512 해시 생성
    let mut hasher = Sha512::new();
    hasher.update(query.as_bytes());
    let query_hash = hex::encode(hasher.finalize());

    let claims = UpbitClaimsWithQuery {
        access_key: access_key.to_string(),
        nonce: Uuid::new_v4().to_string(),
        timestamp: chrono::Utc::now().timestamp_millis(),
        query_hash,
        query_hash_alg: "SHA512".to_string(),
    };

    let header = Header::new(Algorithm::HS256);
    let encoding_key = EncodingKey::from_secret(secret_key.as_bytes());

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

    // ============================================================================
    // JWT with Query Hash Tests
    // ============================================================================

    #[test]
    fn test_jwt_token_with_query_format() {
        // JWT는 header.payload.signature 형식 (3파트)
        let query = r#"{"market":"KRW-BTC","side":"bid","volume":"0.01","price":"100000000","ord_type":"limit"}"#;
        let token = generate_jwt_token_with_query("test_access_key", "test_secret_key", query).unwrap();
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3, "JWT 토큰은 3개의 파트로 구성되어야 합니다");
    }

    #[test]
    fn test_jwt_token_with_query_not_empty() {
        let query = r#"{"market":"KRW-BTC"}"#;
        let token = generate_jwt_token_with_query("test_key", "test_secret", query).unwrap();
        assert!(!token.is_empty(), "JWT 토큰이 비어있으면 안됩니다");
    }

    #[test]
    fn test_jwt_token_with_query_contains_hash() {
        // 토큰 payload에 query_hash 필드가 포함되어야 함
        let query = r#"{"market":"KRW-BTC"}"#;
        let token = generate_jwt_token_with_query("test_key", "test_secret", query).unwrap();

        // JWT payload 디코딩 (base64)
        let parts: Vec<&str> = token.split('.').collect();
        let payload_decoded = base64::Engine::decode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            parts[1],
        )
        .unwrap();
        let payload_str = String::from_utf8(payload_decoded).unwrap();
        let payload: serde_json::Value = serde_json::from_str(&payload_str).unwrap();

        assert!(payload.get("query_hash").is_some(), "payload에 query_hash 필드가 있어야 합니다");
        assert_eq!(payload["query_hash_alg"], "SHA512", "query_hash_alg는 SHA512여야 합니다");
    }

    #[test]
    fn test_jwt_query_hash_is_sha512() {
        // 동일한 쿼리에 대해 동일한 해시가 생성되어야 함
        use sha2::{Digest, Sha512};

        let query = r#"{"market":"KRW-BTC","side":"bid"}"#;
        let mut hasher = Sha512::new();
        hasher.update(query.as_bytes());
        let expected_hash = hex::encode(hasher.finalize());

        let token = generate_jwt_token_with_query("test_key", "test_secret", query).unwrap();
        let parts: Vec<&str> = token.split('.').collect();
        let payload_decoded = base64::Engine::decode(
            &base64::engine::general_purpose::URL_SAFE_NO_PAD,
            parts[1],
        )
        .unwrap();
        let payload_str = String::from_utf8(payload_decoded).unwrap();
        let payload: serde_json::Value = serde_json::from_str(&payload_str).unwrap();

        assert_eq!(
            payload["query_hash"].as_str().unwrap(),
            expected_hash,
            "query_hash가 쿼리의 SHA-512 해시와 일치해야 합니다"
        );
    }

    #[test]
    fn test_jwt_different_query_different_hash() {
        // 다른 쿼리에 대해 다른 해시가 생성되어야 함
        let query1 = r#"{"market":"KRW-BTC"}"#;
        let query2 = r#"{"market":"KRW-ETH"}"#;

        let token1 = generate_jwt_token_with_query("key", "secret", query1).unwrap();
        let token2 = generate_jwt_token_with_query("key", "secret", query2).unwrap();

        // 토큰 payload에서 query_hash 추출
        let get_hash = |token: &str| -> String {
            let parts: Vec<&str> = token.split('.').collect();
            let payload_decoded = base64::Engine::decode(
                &base64::engine::general_purpose::URL_SAFE_NO_PAD,
                parts[1],
            )
            .unwrap();
            let payload_str = String::from_utf8(payload_decoded).unwrap();
            let payload: serde_json::Value = serde_json::from_str(&payload_str).unwrap();
            payload["query_hash"].as_str().unwrap().to_string()
        };

        assert_ne!(
            get_hash(&token1),
            get_hash(&token2),
            "다른 쿼리에 대해 다른 query_hash가 생성되어야 합니다"
        );
    }
}
