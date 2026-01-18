//! WTS Type Definitions
//!
//! TypeScript 타입과 1:1 매칭되는 Rust 타입 정의

use serde::{Deserialize, Serialize};

/// 지원 거래소 (MVP: Upbit)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Exchange {
    Upbit,
}

/// 연결 상태
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
}

/// 로그 레벨
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogLevel {
    Info,
    Success,
    Error,
    Warn,
}

/// 로그 카테고리
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LogCategory {
    Order,
    Balance,
    Deposit,
    Withdraw,
    System,
}

/// 콘솔 로그 엔트리
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleLogEntry {
    /// 고유 ID
    pub id: String,
    /// Unix timestamp (ms)
    pub timestamp: u64,
    /// 로그 레벨
    pub level: LogLevel,
    /// 로그 카테고리
    pub category: LogCategory,
    /// 사용자 친화적 메시지
    pub message: String,
    /// API 응답 원본 (디버깅용)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

/// 주문 유형
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderType {
    Market,
    Limit,
}

/// 주문 방향
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderSide {
    Buy,
    Sell,
}
