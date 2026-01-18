//! CEX (Centralized Exchange) executor.
//!
//! Handles order execution on centralized exchanges like Binance, Coinbase, etc.

use crate::{ExecutorError, ExecutorResult, Order, OrderStatus};
use arbitrage_core::Exchange;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for CEX API clients.
#[async_trait]
pub trait CexClient: Send + Sync {
    /// Submit an order to the exchange.
    async fn submit_order(&self, order: &Order) -> ExecutorResult<String>;

    /// Cancel an order.
    async fn cancel_order(&self, exchange_order_id: &str) -> ExecutorResult<()>;

    /// Get order status.
    async fn get_order_status(&self, exchange_order_id: &str) -> ExecutorResult<OrderStatus>;

    /// Get account balance for an asset.
    async fn get_balance(&self, asset: &str) -> ExecutorResult<u64>;
}

/// Configuration for CEX executor.
#[derive(Debug, Clone)]
pub struct CexExecutorConfig {
    /// Maximum retry attempts.
    pub max_retries: u32,
    /// Retry delay in milliseconds.
    pub retry_delay_ms: u64,
    /// Order timeout in milliseconds.
    pub order_timeout_ms: u64,
    /// Whether to verify balance before order.
    pub verify_balance: bool,
}

impl Default for CexExecutorConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 1000,
            order_timeout_ms: 30000,
            verify_balance: true,
        }
    }
}

/// CEX executor that manages orders on centralized exchanges.
pub struct CexExecutor {
    config: CexExecutorConfig,
    clients: HashMap<Exchange, Arc<dyn CexClient>>,
    pending_orders: Arc<RwLock<HashMap<u64, Order>>>,
}

impl CexExecutor {
    /// Create a new CEX executor.
    pub fn new(config: CexExecutorConfig) -> Self {
        Self {
            config,
            clients: HashMap::new(),
            pending_orders: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a client for an exchange.
    pub fn register_client(&mut self, exchange: Exchange, client: Arc<dyn CexClient>) {
        self.clients.insert(exchange, client);
    }

    /// Get client for an exchange.
    fn get_client(&self, exchange: Exchange) -> ExecutorResult<&Arc<dyn CexClient>> {
        self.clients
            .get(&exchange)
            .ok_or_else(|| ExecutorError::ExchangeError(format!("No client for {:?}", exchange)))
    }

    /// Execute an order.
    pub async fn execute(&self, mut order: Order) -> ExecutorResult<Order> {
        let client = self.get_client(order.exchange)?;

        // Submit order
        let mut retries = 0;
        let exchange_order_id = loop {
            match client.submit_order(&order).await {
                Ok(id) => break id,
                Err(e) => {
                    retries += 1;
                    if retries >= self.config.max_retries {
                        order.fail(&e.to_string());
                        return Err(e);
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(
                        self.config.retry_delay_ms,
                    ))
                    .await;
                }
            }
        };

        order.submit(exchange_order_id);

        // Store pending order
        {
            let mut pending = self.pending_orders.write().await;
            pending.insert(order.id, order.clone());
        }

        Ok(order)
    }

    /// Cancel an order.
    pub async fn cancel(&self, order: &mut Order) -> ExecutorResult<()> {
        if let Some(ref exchange_order_id) = order.exchange_order_id {
            let client = self.get_client(order.exchange)?;
            client.cancel_order(exchange_order_id).await?;
            order.cancel();

            // Remove from pending
            let mut pending = self.pending_orders.write().await;
            pending.remove(&order.id);
        }

        Ok(())
    }

    /// Get pending orders count.
    pub async fn pending_count(&self) -> usize {
        self.pending_orders.read().await.len()
    }
}

/// Mock CEX client for testing.
#[derive(Default)]
pub struct MockCexClient {
    /// Simulated balances.
    pub balances: HashMap<String, u64>,
    /// Should next order fail.
    pub should_fail: bool,
    /// Simulated order counter.
    order_counter: std::sync::atomic::AtomicU64,
}

impl MockCexClient {
    /// Create with default balances.
    pub fn new() -> Self {
        let mut balances = HashMap::new();
        balances.insert("BTC".to_string(), 10_00000000); // 10 BTC
        balances.insert("USDT".to_string(), 100000_00000000); // 100k USDT

        Self {
            balances,
            should_fail: false,
            order_counter: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Set a balance.
    pub fn set_balance(&mut self, asset: &str, amount: u64) {
        self.balances.insert(asset.to_string(), amount);
    }
}

#[async_trait]
impl CexClient for MockCexClient {
    async fn submit_order(&self, _order: &Order) -> ExecutorResult<String> {
        if self.should_fail {
            return Err(ExecutorError::SubmissionFailed("Mock failure".to_string()));
        }

        let id = self
            .order_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(format!("MOCK_{}", id))
    }

    async fn cancel_order(&self, _exchange_order_id: &str) -> ExecutorResult<()> {
        Ok(())
    }

    async fn get_order_status(&self, _exchange_order_id: &str) -> ExecutorResult<OrderStatus> {
        Ok(OrderStatus::Filled)
    }

    async fn get_balance(&self, asset: &str) -> ExecutorResult<u64> {
        self.balances
            .get(asset)
            .copied()
            .ok_or_else(|| ExecutorError::ExchangeError(format!("Unknown asset: {}", asset)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrage_core::TradeSide;

    #[test]
    fn test_cex_executor_config_default() {
        let config = CexExecutorConfig::default();
        assert_eq!(config.max_retries, 3);
        assert!(config.verify_balance);
    }

    #[test]
    fn test_cex_executor_new() {
        let executor = CexExecutor::new(CexExecutorConfig::default());
        assert!(executor.clients.is_empty());
    }

    #[test]
    fn test_cex_executor_register_client() {
        let mut executor = CexExecutor::new(CexExecutorConfig::default());
        let client = Arc::new(MockCexClient::new());
        executor.register_client(Exchange::Binance, client);

        assert!(executor.clients.contains_key(&Exchange::Binance));
    }

    #[tokio::test]
    async fn test_cex_executor_execute() {
        let mut executor = CexExecutor::new(CexExecutorConfig::default());
        let client = Arc::new(MockCexClient::new());
        executor.register_client(Exchange::Binance, client);

        let order = Order::market(Exchange::Binance, 1, TradeSide::Buy, 1_00000000);
        let result = executor.execute(order).await;

        assert!(result.is_ok());
        let order = result.unwrap();
        assert_eq!(order.status, OrderStatus::Submitted);
        assert!(order.exchange_order_id.is_some());
    }

    #[tokio::test]
    async fn test_cex_executor_no_client() {
        let executor = CexExecutor::new(CexExecutorConfig::default());

        let order = Order::market(Exchange::Binance, 1, TradeSide::Buy, 1_00000000);
        let result = executor.execute(order).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cex_executor_pending_count() {
        let mut executor = CexExecutor::new(CexExecutorConfig::default());
        let client = Arc::new(MockCexClient::new());
        executor.register_client(Exchange::Binance, client);

        assert_eq!(executor.pending_count().await, 0);

        let order = Order::market(Exchange::Binance, 1, TradeSide::Buy, 1_00000000);
        executor.execute(order).await.unwrap();

        assert_eq!(executor.pending_count().await, 1);
    }

    #[tokio::test]
    async fn test_mock_cex_client_balance() {
        let client = MockCexClient::new();

        let btc = client.get_balance("BTC").await.unwrap();
        assert_eq!(btc, 10_00000000);

        let usdt = client.get_balance("USDT").await.unwrap();
        assert_eq!(usdt, 100000_00000000);
    }

    #[tokio::test]
    async fn test_mock_cex_client_unknown_asset() {
        let client = MockCexClient::new();
        let result = client.get_balance("UNKNOWN").await;
        assert!(result.is_err());
    }
}
