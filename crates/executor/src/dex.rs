//! DEX (Decentralized Exchange) executor.
//!
//! Handles swap execution on DEXes like Uniswap, SushiSwap, etc.

use crate::{ExecutorError, ExecutorResult, Order};
use arbitrage_core::{Chain, Exchange};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// Swap parameters for DEX execution.
#[derive(Debug, Clone)]
pub struct SwapParams {
    /// DEX to execute on.
    pub exchange: Exchange,
    /// Chain to execute on.
    pub chain: Chain,
    /// Token in address (32 bytes).
    pub token_in: [u8; 32],
    /// Token out address (32 bytes).
    pub token_out: [u8; 32],
    /// Amount in (raw units).
    pub amount_in: u64,
    /// Minimum amount out (slippage protection).
    pub min_amount_out: u64,
    /// Deadline timestamp (seconds).
    pub deadline: u64,
    /// Recipient address.
    pub recipient: [u8; 32],
}

/// Result of a swap execution.
#[derive(Debug, Clone)]
pub struct SwapResult {
    /// Transaction hash.
    pub tx_hash: [u8; 32],
    /// Actual amount received.
    pub amount_out: u64,
    /// Gas used.
    pub gas_used: u64,
    /// Effective gas price.
    pub gas_price: u64,
    /// Block number.
    pub block_number: u64,
}

impl SwapResult {
    /// Calculate gas cost in wei.
    pub fn gas_cost(&self) -> u64 {
        self.gas_used.saturating_mul(self.gas_price)
    }
}

/// Trait for DEX swap clients.
#[async_trait]
pub trait DexClient: Send + Sync {
    /// Execute a swap.
    async fn swap(&self, params: &SwapParams) -> ExecutorResult<SwapResult>;

    /// Get quote for a swap (expected output).
    async fn quote(&self, params: &SwapParams) -> ExecutorResult<u64>;

    /// Check if a pool exists.
    async fn pool_exists(&self, token_a: &[u8; 32], token_b: &[u8; 32]) -> ExecutorResult<bool>;

    /// Get pool liquidity.
    async fn get_liquidity(&self, token_a: &[u8; 32], token_b: &[u8; 32]) -> ExecutorResult<u64>;
}

/// Configuration for DEX executor.
#[derive(Debug, Clone)]
pub struct DexExecutorConfig {
    /// Default slippage tolerance in basis points.
    pub default_slippage_bps: u16,
    /// Transaction deadline offset in seconds.
    pub deadline_offset_secs: u64,
    /// Maximum gas price (in gwei).
    pub max_gas_price_gwei: u64,
    /// Whether to simulate before executing.
    pub simulate_first: bool,
}

impl Default for DexExecutorConfig {
    fn default() -> Self {
        Self {
            default_slippage_bps: 50,  // 0.5%
            deadline_offset_secs: 300, // 5 minutes
            max_gas_price_gwei: 100,
            simulate_first: true,
        }
    }
}

/// DEX executor that manages swaps on decentralized exchanges.
pub struct DexExecutor {
    config: DexExecutorConfig,
    clients: HashMap<(Exchange, Chain), Arc<dyn DexClient>>,
}

impl DexExecutor {
    /// Create a new DEX executor.
    pub fn new(config: DexExecutorConfig) -> Self {
        Self {
            config,
            clients: HashMap::new(),
        }
    }

    /// Register a client for an exchange/chain combination.
    pub fn register_client(
        &mut self,
        exchange: Exchange,
        chain: Chain,
        client: Arc<dyn DexClient>,
    ) {
        self.clients.insert((exchange, chain), client);
    }

    /// Get client for an exchange/chain.
    fn get_client(&self, exchange: Exchange, chain: Chain) -> ExecutorResult<&Arc<dyn DexClient>> {
        self.clients.get(&(exchange, chain)).ok_or_else(|| {
            ExecutorError::ExchangeError(format!("No client for {:?} on {:?}", exchange, chain))
        })
    }

    /// Get a quote for a swap.
    pub async fn quote(&self, params: &SwapParams) -> ExecutorResult<u64> {
        let client = self.get_client(params.exchange, params.chain)?;
        client.quote(params).await
    }

    /// Execute a swap.
    pub async fn swap(&self, params: &SwapParams) -> ExecutorResult<SwapResult> {
        let client = self.get_client(params.exchange, params.chain)?;

        // Optionally simulate first
        if self.config.simulate_first {
            let quote = client.quote(params).await?;
            if quote < params.min_amount_out {
                return Err(ExecutorError::SlippageExceeded {
                    expected_bps: self.config.default_slippage_bps,
                    actual_bps: calculate_slippage_bps(params.amount_in, quote),
                });
            }
        }

        client.swap(params).await
    }

    /// Check if swap route is available.
    pub async fn route_available(
        &self,
        exchange: Exchange,
        chain: Chain,
        token_a: &[u8; 32],
        token_b: &[u8; 32],
    ) -> ExecutorResult<bool> {
        let client = self.get_client(exchange, chain)?;
        client.pool_exists(token_a, token_b).await
    }

    /// Build swap params from an order.
    pub fn build_swap_params(
        &self,
        order: &Order,
        chain: Chain,
        token_in: [u8; 32],
        token_out: [u8; 32],
        recipient: [u8; 32],
    ) -> SwapParams {
        let deadline = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + self.config.deadline_offset_secs;

        // Calculate min_amount_out with slippage
        let slippage_bps = order.max_slippage_bps.max(self.config.default_slippage_bps);
        let min_amount_out = if order.price > 0 {
            // Use u128 to prevent overflow
            let expected = (order.quantity as u128 * order.price as u128 / 100000000) as u64;
            expected * (10000 - slippage_bps as u64) / 10000
        } else {
            0
        };

        SwapParams {
            exchange: order.exchange,
            chain,
            token_in,
            token_out,
            amount_in: order.quantity,
            min_amount_out,
            deadline,
            recipient,
        }
    }
}

fn calculate_slippage_bps(expected: u64, actual: u64) -> u16 {
    if expected == 0 {
        return 0;
    }
    let diff = expected.saturating_sub(actual);
    ((diff as u128 * 10000) / expected as u128) as u16
}

/// Mock DEX client for testing.
#[derive(Default)]
pub struct MockDexClient {
    /// Simulated liquidity.
    pub liquidity: u64,
    /// Price ratio (output / input, in basis points where 10000 = 1:1).
    pub price_ratio_bps: u64,
    /// Should next swap fail.
    pub should_fail: bool,
}

impl MockDexClient {
    /// Create with default settings.
    pub fn new() -> Self {
        Self {
            liquidity: 1000000_00000000, // 1M
            price_ratio_bps: 10000,      // 1:1
            should_fail: false,
        }
    }

    /// Set price ratio.
    pub fn with_price_ratio(mut self, ratio_bps: u64) -> Self {
        self.price_ratio_bps = ratio_bps;
        self
    }
}

#[async_trait]
impl DexClient for MockDexClient {
    async fn swap(&self, params: &SwapParams) -> ExecutorResult<SwapResult> {
        if self.should_fail {
            return Err(ExecutorError::ExchangeError("Mock failure".to_string()));
        }

        let amount_out = params.amount_in * self.price_ratio_bps / 10000;

        if amount_out < params.min_amount_out {
            return Err(ExecutorError::SlippageExceeded {
                expected_bps: 50,
                actual_bps: calculate_slippage_bps(params.min_amount_out, amount_out),
            });
        }

        Ok(SwapResult {
            tx_hash: [0u8; 32],
            amount_out,
            gas_used: 150000,
            gas_price: 30_000000000, // 30 gwei
            block_number: 12345678,
        })
    }

    async fn quote(&self, params: &SwapParams) -> ExecutorResult<u64> {
        Ok(params.amount_in * self.price_ratio_bps / 10000)
    }

    async fn pool_exists(&self, _token_a: &[u8; 32], _token_b: &[u8; 32]) -> ExecutorResult<bool> {
        Ok(self.liquidity > 0)
    }

    async fn get_liquidity(&self, _token_a: &[u8; 32], _token_b: &[u8; 32]) -> ExecutorResult<u64> {
        Ok(self.liquidity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arbitrage_core::TradeSide;

    #[test]
    fn test_dex_executor_config_default() {
        let config = DexExecutorConfig::default();
        assert_eq!(config.default_slippage_bps, 50);
        assert!(config.simulate_first);
    }

    #[test]
    fn test_dex_executor_new() {
        let executor = DexExecutor::new(DexExecutorConfig::default());
        assert!(executor.clients.is_empty());
    }

    #[test]
    fn test_dex_executor_register_client() {
        let mut executor = DexExecutor::new(DexExecutorConfig::default());
        let client = Arc::new(MockDexClient::new());
        executor.register_client(Exchange::UniswapV3, Chain::Ethereum, client);

        assert!(executor
            .clients
            .contains_key(&(Exchange::UniswapV3, Chain::Ethereum)));
    }

    #[test]
    fn test_swap_params() {
        let params = SwapParams {
            exchange: Exchange::UniswapV3,
            chain: Chain::Ethereum,
            token_in: [0u8; 32],
            token_out: [1u8; 32],
            amount_in: 1_00000000,
            min_amount_out: 99000000,
            deadline: 1700000000,
            recipient: [2u8; 32],
        };

        assert_eq!(params.exchange, Exchange::UniswapV3);
        assert_eq!(params.amount_in, 1_00000000);
    }

    #[test]
    fn test_swap_result_gas_cost() {
        let result = SwapResult {
            tx_hash: [0u8; 32],
            amount_out: 1_00000000,
            gas_used: 150000,
            gas_price: 30_000000000, // 30 gwei
            block_number: 12345678,
        };

        // 150000 * 30 gwei = 4500000 gwei = 0.0045 ETH
        assert_eq!(result.gas_cost(), 4500000_000000000);
    }

    #[tokio::test]
    async fn test_dex_executor_quote() {
        let mut executor = DexExecutor::new(DexExecutorConfig::default());
        let client = Arc::new(MockDexClient::new().with_price_ratio(10000)); // 1:1
        executor.register_client(Exchange::UniswapV3, Chain::Ethereum, client);

        let params = SwapParams {
            exchange: Exchange::UniswapV3,
            chain: Chain::Ethereum,
            token_in: [0u8; 32],
            token_out: [1u8; 32],
            amount_in: 1_00000000,
            min_amount_out: 0,
            deadline: u64::MAX,
            recipient: [0u8; 32],
        };

        let quote = executor.quote(&params).await.unwrap();
        assert_eq!(quote, 1_00000000);
    }

    #[tokio::test]
    async fn test_dex_executor_swap() {
        let mut executor = DexExecutor::new(DexExecutorConfig {
            simulate_first: false,
            ..Default::default()
        });
        let client = Arc::new(MockDexClient::new());
        executor.register_client(Exchange::UniswapV3, Chain::Ethereum, client);

        let params = SwapParams {
            exchange: Exchange::UniswapV3,
            chain: Chain::Ethereum,
            token_in: [0u8; 32],
            token_out: [1u8; 32],
            amount_in: 1_00000000,
            min_amount_out: 99000000,
            deadline: u64::MAX,
            recipient: [0u8; 32],
        };

        let result = executor.swap(&params).await.unwrap();
        assert_eq!(result.amount_out, 1_00000000);
    }

    #[tokio::test]
    async fn test_dex_executor_no_client() {
        let executor = DexExecutor::new(DexExecutorConfig::default());

        let params = SwapParams {
            exchange: Exchange::UniswapV3,
            chain: Chain::Ethereum,
            token_in: [0u8; 32],
            token_out: [1u8; 32],
            amount_in: 1_00000000,
            min_amount_out: 0,
            deadline: u64::MAX,
            recipient: [0u8; 32],
        };

        let result = executor.quote(&params).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_dex_client_pool_exists() {
        let client = MockDexClient::new();
        let exists = client.pool_exists(&[0u8; 32], &[1u8; 32]).await.unwrap();
        assert!(exists);
    }

    #[tokio::test]
    async fn test_mock_dex_client_slippage() {
        let client = MockDexClient::new().with_price_ratio(9000); // 10% slippage

        let params = SwapParams {
            exchange: Exchange::UniswapV3,
            chain: Chain::Ethereum,
            token_in: [0u8; 32],
            token_out: [1u8; 32],
            amount_in: 1_00000000,
            min_amount_out: 95000000, // Expect at least 0.95
            deadline: u64::MAX,
            recipient: [0u8; 32],
        };

        let result = client.swap(&params).await;
        assert!(result.is_err()); // Should fail due to slippage
    }

    #[test]
    fn test_build_swap_params() {
        let executor = DexExecutor::new(DexExecutorConfig::default());
        let order = Order::limit(
            Exchange::UniswapV3,
            1,
            TradeSide::Buy,
            1_00000000,
            50000_00000000,
        );

        let params = executor.build_swap_params(
            &order,
            Chain::Ethereum,
            [0u8; 32],
            [1u8; 32],
            [2u8; 32],
        );

        assert_eq!(params.exchange, Exchange::UniswapV3);
        assert_eq!(params.amount_in, 1_00000000);
        assert!(params.deadline > 0);
    }

    #[test]
    fn test_calculate_slippage_bps() {
        assert_eq!(calculate_slippage_bps(100, 99), 100);  // 1%
        assert_eq!(calculate_slippage_bps(100, 95), 500);  // 5%
        assert_eq!(calculate_slippage_bps(100, 100), 0);   // 0%
        assert_eq!(calculate_slippage_bps(0, 100), 0);     // Edge case
    }
}
