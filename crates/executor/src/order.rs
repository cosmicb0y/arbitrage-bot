//! Order types and state management.

use arbitrage_core::{Exchange, TradeSide};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};

static ORDER_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Order status in the execution lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum OrderStatus {
    /// Order created but not yet submitted.
    Pending = 0,
    /// Order submitted to exchange.
    Submitted = 1,
    /// Order partially filled.
    PartiallyFilled = 2,
    /// Order completely filled.
    Filled = 3,
    /// Order cancelled by user or system.
    Cancelled = 4,
    /// Order failed due to error.
    Failed = 5,
    /// Order expired.
    Expired = 6,
}

impl OrderStatus {
    /// Check if order is in a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            OrderStatus::Filled
                | OrderStatus::Cancelled
                | OrderStatus::Failed
                | OrderStatus::Expired
        )
    }

    /// Check if order is active (can still be filled).
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            OrderStatus::Pending | OrderStatus::Submitted | OrderStatus::PartiallyFilled
        )
    }
}

/// Order type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum OrderType {
    /// Market order - execute at current price.
    Market = 0,
    /// Limit order - execute at specified price or better.
    Limit = 1,
    /// Immediate or cancel - fill what you can, cancel rest.
    Ioc = 2,
    /// Fill or kill - fill entire order or cancel.
    Fok = 3,
}

/// Time in force for orders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum TimeInForce {
    /// Good till cancelled.
    Gtc = 0,
    /// Immediate or cancel.
    Ioc = 1,
    /// Fill or kill.
    Fok = 2,
    /// Good till date.
    Gtd = 3,
}

/// An order to be executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Internal order ID.
    pub id: u64,
    /// Exchange order ID (after submission).
    pub exchange_order_id: Option<String>,
    /// Target exchange.
    pub exchange: Exchange,
    /// Trading pair ID.
    pub pair_id: u32,
    /// Buy or sell.
    pub side: TradeSide,
    /// Order type.
    pub order_type: OrderType,
    /// Order quantity (in base asset, fixed-point 8 decimals).
    pub quantity: u64,
    /// Limit price (fixed-point 8 decimals, 0 for market).
    pub price: u64,
    /// Filled quantity so far.
    pub filled_quantity: u64,
    /// Average fill price.
    pub avg_fill_price: u64,
    /// Current status.
    pub status: OrderStatus,
    /// Maximum allowed slippage in basis points.
    pub max_slippage_bps: u16,
    /// Creation timestamp (ms).
    pub created_at_ms: u64,
    /// Last update timestamp (ms).
    pub updated_at_ms: u64,
    /// Error message if failed.
    pub error_message: Option<String>,
}

impl Order {
    /// Create a new market order.
    pub fn market(exchange: Exchange, pair_id: u32, side: TradeSide, quantity: u64) -> Self {
        let now = current_time_ms();
        Self {
            id: ORDER_ID_COUNTER.fetch_add(1, Ordering::SeqCst),
            exchange_order_id: None,
            exchange,
            pair_id,
            side,
            order_type: OrderType::Market,
            quantity,
            price: 0,
            filled_quantity: 0,
            avg_fill_price: 0,
            status: OrderStatus::Pending,
            max_slippage_bps: 50, // 0.5% default
            created_at_ms: now,
            updated_at_ms: now,
            error_message: None,
        }
    }

    /// Create a new limit order.
    pub fn limit(
        exchange: Exchange,
        pair_id: u32,
        side: TradeSide,
        quantity: u64,
        price: u64,
    ) -> Self {
        let now = current_time_ms();
        Self {
            id: ORDER_ID_COUNTER.fetch_add(1, Ordering::SeqCst),
            exchange_order_id: None,
            exchange,
            pair_id,
            side,
            order_type: OrderType::Limit,
            quantity,
            price,
            filled_quantity: 0,
            avg_fill_price: 0,
            status: OrderStatus::Pending,
            max_slippage_bps: 50,
            created_at_ms: now,
            updated_at_ms: now,
            error_message: None,
        }
    }

    /// Set maximum slippage.
    pub fn with_slippage(mut self, bps: u16) -> Self {
        self.max_slippage_bps = bps;
        self
    }

    /// Mark order as submitted.
    pub fn submit(&mut self, exchange_order_id: String) {
        self.exchange_order_id = Some(exchange_order_id);
        self.status = OrderStatus::Submitted;
        self.updated_at_ms = current_time_ms();
    }

    /// Update fill status.
    pub fn fill(&mut self, filled_qty: u64, fill_price: u64) {
        // Update average price
        let total_value = self.avg_fill_price as u128 * self.filled_quantity as u128
            + fill_price as u128 * filled_qty as u128;
        self.filled_quantity += filled_qty;

        if self.filled_quantity > 0 {
            self.avg_fill_price = (total_value / self.filled_quantity as u128) as u64;
        }

        // Update status
        if self.filled_quantity >= self.quantity {
            self.status = OrderStatus::Filled;
        } else {
            self.status = OrderStatus::PartiallyFilled;
        }

        self.updated_at_ms = current_time_ms();
    }

    /// Cancel the order.
    pub fn cancel(&mut self) {
        self.status = OrderStatus::Cancelled;
        self.updated_at_ms = current_time_ms();
    }

    /// Mark as failed.
    pub fn fail(&mut self, reason: &str) {
        self.status = OrderStatus::Failed;
        self.error_message = Some(reason.to_string());
        self.updated_at_ms = current_time_ms();
    }

    /// Calculate fill percentage.
    pub fn fill_percent(&self) -> u8 {
        if self.quantity == 0 {
            return 0;
        }
        ((self.filled_quantity as u128 * 100) / self.quantity as u128) as u8
    }

    /// Calculate remaining quantity.
    pub fn remaining(&self) -> u64 {
        self.quantity.saturating_sub(self.filled_quantity)
    }

    /// Check if fully filled.
    pub fn is_filled(&self) -> bool {
        self.status == OrderStatus::Filled
    }
}

/// Order fill event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderFill {
    /// Order ID.
    pub order_id: u64,
    /// Fill quantity.
    pub quantity: u64,
    /// Fill price.
    pub price: u64,
    /// Fill timestamp.
    pub timestamp_ms: u64,
    /// Trade ID from exchange.
    pub trade_id: Option<String>,
    /// Fee paid.
    pub fee: u64,
    /// Fee asset.
    pub fee_asset: String,
}

/// Execution result for an opportunity.
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Opportunity ID that was executed.
    pub opportunity_id: u64,
    /// Orders created for this execution.
    pub orders: Vec<Order>,
    /// Net profit/loss (in quote asset, can be negative).
    pub realized_pnl: i64,
    /// Total fees paid.
    pub total_fees: u64,
    /// Execution start time.
    pub started_at_ms: u64,
    /// Execution end time.
    pub completed_at_ms: Option<u64>,
    /// Whether execution was successful.
    pub success: bool,
    /// Error message if failed.
    pub error: Option<String>,
}

impl ExecutionResult {
    /// Create a new execution result.
    pub fn new(opportunity_id: u64) -> Self {
        Self {
            opportunity_id,
            orders: Vec::new(),
            realized_pnl: 0,
            total_fees: 0,
            started_at_ms: current_time_ms(),
            completed_at_ms: None,
            success: false,
            error: None,
        }
    }

    /// Add an order to the result.
    pub fn add_order(&mut self, order: Order) {
        self.orders.push(order);
    }

    /// Mark as complete.
    pub fn complete(&mut self, pnl: i64, fees: u64) {
        self.realized_pnl = pnl;
        self.total_fees = fees;
        self.completed_at_ms = Some(current_time_ms());
        self.success = true;
    }

    /// Mark as failed.
    pub fn fail(&mut self, reason: &str) {
        self.completed_at_ms = Some(current_time_ms());
        self.success = false;
        self.error = Some(reason.to_string());
    }

    /// Calculate execution duration in ms.
    pub fn duration_ms(&self) -> Option<u64> {
        self.completed_at_ms.map(|end| end - self.started_at_ms)
    }
}

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_status_is_terminal() {
        assert!(!OrderStatus::Pending.is_terminal());
        assert!(!OrderStatus::Submitted.is_terminal());
        assert!(!OrderStatus::PartiallyFilled.is_terminal());
        assert!(OrderStatus::Filled.is_terminal());
        assert!(OrderStatus::Cancelled.is_terminal());
        assert!(OrderStatus::Failed.is_terminal());
        assert!(OrderStatus::Expired.is_terminal());
    }

    #[test]
    fn test_order_status_is_active() {
        assert!(OrderStatus::Pending.is_active());
        assert!(OrderStatus::Submitted.is_active());
        assert!(OrderStatus::PartiallyFilled.is_active());
        assert!(!OrderStatus::Filled.is_active());
        assert!(!OrderStatus::Cancelled.is_active());
    }

    #[test]
    fn test_order_market() {
        let order = Order::market(Exchange::Binance, 1, TradeSide::Buy, 1_00000000);

        assert_eq!(order.exchange, Exchange::Binance);
        assert_eq!(order.side, TradeSide::Buy);
        assert_eq!(order.quantity, 1_00000000);
        assert_eq!(order.order_type, OrderType::Market);
        assert_eq!(order.status, OrderStatus::Pending);
    }

    #[test]
    fn test_order_limit() {
        let order = Order::limit(
            Exchange::Coinbase,
            1,
            TradeSide::Sell,
            2_00000000,
            50000_00000000,
        );

        assert_eq!(order.order_type, OrderType::Limit);
        assert_eq!(order.price, 50000_00000000);
    }

    #[test]
    fn test_order_with_slippage() {
        let order =
            Order::market(Exchange::Binance, 1, TradeSide::Buy, 1_00000000).with_slippage(100); // 1%

        assert_eq!(order.max_slippage_bps, 100);
    }

    #[test]
    fn test_order_submit() {
        let mut order = Order::market(Exchange::Binance, 1, TradeSide::Buy, 1_00000000);
        order.submit("EX123".to_string());

        assert_eq!(order.status, OrderStatus::Submitted);
        assert_eq!(order.exchange_order_id, Some("EX123".to_string()));
    }

    #[test]
    fn test_order_fill_partial() {
        let mut order = Order::market(Exchange::Binance, 1, TradeSide::Buy, 2_00000000);
        order.submit("EX123".to_string());
        order.fill(1_00000000, 50000_00000000);

        assert_eq!(order.status, OrderStatus::PartiallyFilled);
        assert_eq!(order.filled_quantity, 1_00000000);
        assert_eq!(order.avg_fill_price, 50000_00000000);
        assert_eq!(order.fill_percent(), 50);
    }

    #[test]
    fn test_order_fill_complete() {
        let mut order = Order::market(Exchange::Binance, 1, TradeSide::Buy, 2_00000000);
        order.submit("EX123".to_string());
        order.fill(1_00000000, 50000_00000000);
        order.fill(1_00000000, 50100_00000000);

        assert_eq!(order.status, OrderStatus::Filled);
        assert_eq!(order.filled_quantity, 2_00000000);
        // Average: (50000 + 50100) / 2 = 50050
        assert_eq!(order.avg_fill_price, 50050_00000000);
        assert!(order.is_filled());
    }

    #[test]
    fn test_order_cancel() {
        let mut order = Order::market(Exchange::Binance, 1, TradeSide::Buy, 1_00000000);
        order.submit("EX123".to_string());
        order.cancel();

        assert_eq!(order.status, OrderStatus::Cancelled);
    }

    #[test]
    fn test_order_fail() {
        let mut order = Order::market(Exchange::Binance, 1, TradeSide::Buy, 1_00000000);
        order.fail("Insufficient balance");

        assert_eq!(order.status, OrderStatus::Failed);
        assert_eq!(
            order.error_message,
            Some("Insufficient balance".to_string())
        );
    }

    #[test]
    fn test_order_remaining() {
        let mut order = Order::market(Exchange::Binance, 1, TradeSide::Buy, 2_00000000);
        order.fill(50000000, 50000_00000000);

        assert_eq!(order.remaining(), 1_50000000);
    }

    #[test]
    fn test_execution_result_new() {
        let result = ExecutionResult::new(123);

        assert_eq!(result.opportunity_id, 123);
        assert!(result.orders.is_empty());
        assert!(!result.success);
    }

    #[test]
    fn test_execution_result_complete() {
        let mut result = ExecutionResult::new(123);
        result.complete(1000, 50);

        assert!(result.success);
        assert_eq!(result.realized_pnl, 1000);
        assert_eq!(result.total_fees, 50);
        assert!(result.duration_ms().is_some());
    }

    #[test]
    fn test_execution_result_fail() {
        let mut result = ExecutionResult::new(123);
        result.fail("Order rejected");

        assert!(!result.success);
        assert_eq!(result.error, Some("Order rejected".to_string()));
    }
}
