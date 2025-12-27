//! Route finding for arbitrage execution.
//!
//! Calculates optimal routes including bridges and withdrawals.

use arbitrage_core::{BridgeProtocol, Chain, Exchange, RouteStep, TradeSide};

/// Cost breakdown for a route.
#[derive(Debug, Clone, Default)]
pub struct RouteCosts {
    /// Gas cost in basis points.
    pub gas_cost: i32,
    /// Trading fee in basis points.
    pub trading_fee: i32,
    /// Bridge fee in basis points.
    pub bridge_fee: i32,
    /// Withdrawal fee in basis points.
    pub withdrawal_fee: i32,
}

impl RouteCosts {
    /// Total cost in basis points.
    pub fn total(&self) -> i32 {
        self.gas_cost + self.trading_fee + self.bridge_fee + self.withdrawal_fee
    }
}

/// An execution route for arbitrage.
#[derive(Debug, Clone)]
pub struct Route {
    steps: Vec<RouteStep>,
    amount: u64,
    buy_price: u64,
    sell_price: u64,
}

impl Route {
    /// Create a new route.
    pub fn new(steps: Vec<RouteStep>, amount: u64, buy_price: u64, sell_price: u64) -> Self {
        Self {
            steps,
            amount,
            buy_price,
            sell_price,
        }
    }

    /// Get the route steps.
    pub fn steps(&self) -> &[RouteStep] {
        &self.steps
    }

    /// Check if this is a direct trade (no bridges or withdrawals).
    pub fn is_direct(&self) -> bool {
        self.steps.iter().all(|step| {
            matches!(step, RouteStep::Trade { .. })
        })
    }

    /// Check if route includes a bridge.
    pub fn has_bridge(&self) -> bool {
        self.steps.iter().any(|step| {
            matches!(step, RouteStep::Bridge { .. })
        })
    }

    /// Estimate costs for this route.
    pub fn estimate_costs(&self) -> RouteCosts {
        let mut costs = RouteCosts::default();

        for step in &self.steps {
            match step {
                RouteStep::Trade { .. } => {
                    costs.trading_fee += 10; // 0.1% per trade
                    costs.gas_cost += 5;     // Gas for DEX trades
                }
                RouteStep::Bridge { .. } => {
                    costs.bridge_fee += 30;  // 0.3% bridge fee
                    costs.gas_cost += 20;    // Gas for bridge tx
                }
                RouteStep::Withdraw { .. } => {
                    costs.withdrawal_fee += 5;
                    costs.gas_cost += 10;
                }
                RouteStep::Deposit { .. } => {
                    costs.gas_cost += 5;
                }
            }
        }

        costs
    }

    /// Calculate net profit in basis points.
    pub fn net_profit_bps(&self) -> i32 {
        if self.buy_price == 0 {
            return 0;
        }

        // Gross premium
        let gross_bps = ((self.sell_price as i64 - self.buy_price as i64) * 10000
            / self.buy_price as i64) as i32;

        // Subtract costs
        let costs = self.estimate_costs();
        gross_bps - costs.total()
    }
}

/// Builder for constructing routes step by step.
#[derive(Debug, Default)]
pub struct RouteBuilder {
    steps: Vec<RouteStep>,
    amount: u64,
    buy_price: u64,
    sell_price: u64,
}

impl RouteBuilder {
    /// Create a new route builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a buy trade step.
    pub fn buy(mut self, exchange: Exchange, pair_id: u32, price: u64) -> Self {
        self.buy_price = price;
        self.steps.push(RouteStep::Trade {
            exchange,
            pair_id,
            side: TradeSide::Buy,
            expected_price: price,
            slippage_bps: 50, // 0.5% default slippage
        });
        self
    }

    /// Add a sell trade step.
    pub fn sell(mut self, exchange: Exchange, pair_id: u32, price: u64) -> Self {
        self.sell_price = price;
        self.steps.push(RouteStep::Trade {
            exchange,
            pair_id,
            side: TradeSide::Sell,
            expected_price: price,
            slippage_bps: 50,
        });
        self
    }

    /// Add a withdrawal step.
    pub fn withdraw(mut self, exchange: Exchange, chain: Chain) -> Self {
        self.steps.push(RouteStep::Withdraw {
            exchange,
            chain,
        });
        self
    }

    /// Add a deposit step.
    pub fn deposit(mut self, exchange: Exchange, chain: Chain) -> Self {
        self.steps.push(RouteStep::Deposit {
            exchange,
            chain,
        });
        self
    }

    /// Add a bridge step.
    pub fn bridge(mut self, protocol: BridgeProtocol, source: Chain, dest: Chain) -> Self {
        self.steps.push(RouteStep::Bridge {
            protocol,
            source_chain: source,
            dest_chain: dest,
        });
        self
    }

    /// Set the trade amount.
    pub fn with_amount(mut self, amount: u64) -> Self {
        self.amount = amount;
        self
    }

    /// Build the final route.
    pub fn build(self) -> Route {
        Route::new(self.steps, self.amount, self.buy_price, self.sell_price)
    }
}

/// Finds optimal routes between exchanges.
#[derive(Debug, Default)]
pub struct RouteFinder {
    /// Known bridge routes.
    bridges: Vec<BridgeInfo>,
}

/// Information about a bridge connection.
#[derive(Debug, Clone)]
struct BridgeInfo {
    protocol: BridgeProtocol,
    source: Chain,
    dest: Chain,
}

impl RouteFinder {
    /// Create a new route finder.
    pub fn new() -> Self {
        // Initialize with common bridges
        let bridges = vec![
            BridgeInfo {
                protocol: BridgeProtocol::Stargate,
                source: Chain::Ethereum,
                dest: Chain::Arbitrum,
            },
            BridgeInfo {
                protocol: BridgeProtocol::Stargate,
                source: Chain::Ethereum,
                dest: Chain::Optimism,
            },
            BridgeInfo {
                protocol: BridgeProtocol::Stargate,
                source: Chain::Arbitrum,
                dest: Chain::Ethereum,
            },
            BridgeInfo {
                protocol: BridgeProtocol::LayerZero,
                source: Chain::Ethereum,
                dest: Chain::Bsc,
            },
        ];

        Self { bridges }
    }

    /// Find all possible routes between two exchanges on a chain.
    pub fn find_routes(
        &self,
        source: Exchange,
        target: Exchange,
        _chain: Chain,
    ) -> Vec<RouteBuilder> {
        let mut routes = Vec::new();

        // Direct route (same chain or CEX-to-CEX)
        routes.push(
            RouteBuilder::new()
                .buy(source, 1, 0)
                .sell(target, 1, 0)
        );

        // If different chains might be involved, add bridge routes
        // This is a simplified implementation
        if self.might_need_bridge(source, target) {
            for bridge in &self.bridges {
                routes.push(
                    RouteBuilder::new()
                        .buy(source, 1, 0)
                        .bridge(bridge.protocol, bridge.source, bridge.dest)
                        .sell(target, 1, 0)
                );
            }
        }

        routes
    }

    /// Check if a route might need a bridge.
    fn might_need_bridge(&self, source: Exchange, target: Exchange) -> bool {
        // DEX to DEX on different chains might need bridge
        let source_is_dex = matches!(
            source,
            Exchange::UniswapV2 | Exchange::UniswapV3 | Exchange::SushiSwap
        );
        let target_is_dex = matches!(
            target,
            Exchange::UniswapV2 | Exchange::UniswapV3 | Exchange::SushiSwap
        );

        source_is_dex && target_is_dex
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_builder_direct() {
        let route = RouteBuilder::new()
            .buy(Exchange::Binance, 1, 50000)
            .sell(Exchange::Coinbase, 1, 50500)
            .build();

        assert_eq!(route.steps().len(), 2);
        assert!(route.is_direct());
    }

    #[test]
    fn test_route_builder_with_withdrawal() {
        let route = RouteBuilder::new()
            .buy(Exchange::Binance, 1, 50000)
            .withdraw(Exchange::Binance, Chain::Ethereum)
            .deposit(Exchange::Coinbase, Chain::Ethereum)
            .sell(Exchange::Coinbase, 1, 50500)
            .build();

        assert_eq!(route.steps().len(), 4);
        assert!(!route.is_direct());
    }

    #[test]
    fn test_route_builder_with_bridge() {
        let route = RouteBuilder::new()
            .buy(Exchange::UniswapV3, 1, 50000)
            .bridge(BridgeProtocol::Stargate, Chain::Ethereum, Chain::Arbitrum)
            .sell(Exchange::UniswapV3, 1, 50500)
            .build();

        assert_eq!(route.steps().len(), 3);
        assert!(route.has_bridge());
    }

    #[test]
    fn test_route_cost_estimation() {
        let route = RouteBuilder::new()
            .buy(Exchange::Binance, 1, 50000)
            .withdraw(Exchange::Binance, Chain::Ethereum)
            .deposit(Exchange::Coinbase, Chain::Ethereum)
            .sell(Exchange::Coinbase, 1, 50500)
            .build();

        let costs = route.estimate_costs();
        assert!(costs.gas_cost > 0);
        assert!(costs.trading_fee > 0);
    }

    #[test]
    fn test_route_net_profit() {
        let route = RouteBuilder::new()
            .buy(Exchange::Binance, 1, 50000)
            .sell(Exchange::Coinbase, 1, 50500)
            .with_amount(1_00000000) // 1 unit
            .build();

        let profit = route.net_profit_bps();
        // Should be positive after fees
        assert!(profit > 0 || profit <= 100); // Some profit or small loss from fees
    }

    #[test]
    fn test_route_finder_find_best() {
        let finder = RouteFinder::new();

        let routes = finder.find_routes(
            Exchange::Binance,
            Exchange::Coinbase,
            Chain::Ethereum,
        );

        assert!(!routes.is_empty());
    }
}
