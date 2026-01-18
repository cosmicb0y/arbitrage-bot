//! Orderbook depth walking algorithm for optimal arbitrage size calculation.
//!
//! This module implements the depth-walking algorithm that calculates the
//! maximum profitable trade size given two orderbooks and fee structures.

use arbitrage_core::FixedPoint;

/// Result of optimal size calculation.
#[derive(Debug, Clone, Copy, Default)]
pub struct OptimalSizeResult {
    /// Maximum profitable quantity (in base asset, FixedPoint scale).
    pub amount: u64,
    /// Total profit at this amount (in quote currency, FixedPoint scale).
    /// This is gross profit minus fees.
    pub profit: i64,
    /// Effective average buy price (weighted by quantity).
    pub avg_buy_price: u64,
    /// Effective average sell price (weighted by quantity).
    pub avg_sell_price: u64,
    /// Number of buy orderbook levels consumed.
    pub levels_consumed_buy: usize,
    /// Number of sell orderbook levels consumed.
    pub levels_consumed_sell: usize,
}

impl OptimalSizeResult {
    /// Check if this result represents a profitable trade.
    pub fn is_profitable(&self) -> bool {
        self.profit > 0 && self.amount > 0
    }

    /// Get profit in basis points relative to notional value.
    pub fn profit_bps(&self) -> i32 {
        if self.amount == 0 || self.avg_buy_price == 0 {
            return 0;
        }
        // profit / (amount * avg_buy_price) * 10000
        let notional =
            (self.amount as i128 * self.avg_buy_price as i128) / FixedPoint::SCALE as i128;
        if notional == 0 {
            return 0;
        }
        ((self.profit as i128 * 10000) / notional) as i32
    }
}

/// Fee configuration for optimal size calculation.
#[derive(Debug, Clone, Copy)]
pub struct DepthFeeConfig {
    /// Buy (taker) fee in basis points (e.g., 10 = 0.1%).
    pub buy_fee_bps: u32,
    /// Sell (taker) fee in basis points.
    pub sell_fee_bps: u32,
    /// Fixed withdrawal fee in base asset units (FixedPoint scale).
    /// This is subtracted from profit as a fixed cost.
    pub withdrawal_fee: u64,
}

impl Default for DepthFeeConfig {
    fn default() -> Self {
        Self {
            buy_fee_bps: 10, // 0.1% default
            sell_fee_bps: 10,
            withdrawal_fee: 0,
        }
    }
}

/// Calculate optimal arbitrage size using depth-walking algorithm.
///
/// This function walks through both orderbooks simultaneously to find
/// the maximum quantity that can be profitably traded considering:
/// - Multiple price levels (depth)
/// - Trading fees on both sides
/// - Fixed withdrawal costs
///
/// # Algorithm
/// 1. Start at the best ask (buy side) and best bid (sell side)
/// 2. Calculate effective prices after fees
/// 3. While sell price > buy price (profitable):
///    - Trade the minimum of remaining quantities at current levels
///    - Accumulate amount and profit
///    - Advance to next level when current is exhausted
/// 4. Subtract fixed costs from total profit
///
/// # Arguments
/// * `buy_asks` - Asks from the exchange where we BUY (ascending by price)
/// * `sell_bids` - Bids from the exchange where we SELL (descending by price)
/// * `fees` - Fee configuration
///
/// # Returns
/// `OptimalSizeResult` with the maximum profitable amount and expected profit.
pub fn calculate_optimal_size(
    buy_asks: &[(u64, u64)],
    sell_bids: &[(u64, u64)],
    fees: DepthFeeConfig,
) -> OptimalSizeResult {
    if buy_asks.is_empty() || sell_bids.is_empty() {
        return OptimalSizeResult::default();
    }

    let mut i = 0usize; // asks pointer (buy side)
    let mut j = 0usize; // bids pointer (sell side)

    // Remaining quantities at current levels
    let mut buy_remaining = buy_asks[0].1;
    let mut sell_remaining = sell_bids[0].1;

    let mut total_amount: u64 = 0;
    let mut total_profit: i128 = 0; // Use i128 to avoid overflow
    let mut total_buy_cost: u128 = 0;
    let mut total_sell_revenue: u128 = 0;

    while i < buy_asks.len() && j < sell_bids.len() {
        let (buy_price, _) = buy_asks[i];
        let (sell_price, _) = sell_bids[j];

        // Calculate effective prices after fees
        // effective_buy = price * (1 + fee_bps / 10000)
        // effective_sell = price * (1 - fee_bps / 10000)
        let effective_buy = buy_price as u128 * (10000 + fees.buy_fee_bps as u128) / 10000;
        let effective_sell = sell_price as u128 * (10000 - fees.sell_fee_bps as u128) / 10000;

        // Check if still profitable
        if effective_sell <= effective_buy {
            break;
        }

        // Trade minimum of remaining quantities at each level
        let qty = buy_remaining.min(sell_remaining);
        if qty == 0 {
            break;
        }

        // Accumulate
        total_amount = total_amount.saturating_add(qty);

        // Calculate profit for this quantity
        // profit = qty * (effective_sell - effective_buy) / SCALE
        let level_profit = (qty as i128 * (effective_sell as i128 - effective_buy as i128))
            / FixedPoint::SCALE as i128;
        total_profit += level_profit;

        // Track costs for average price calculation
        total_buy_cost += (qty as u128 * buy_price as u128) / FixedPoint::SCALE as u128;
        total_sell_revenue += (qty as u128 * sell_price as u128) / FixedPoint::SCALE as u128;

        // Advance pointers
        buy_remaining = buy_remaining.saturating_sub(qty);
        sell_remaining = sell_remaining.saturating_sub(qty);

        if buy_remaining == 0 {
            i += 1;
            if i < buy_asks.len() {
                buy_remaining = buy_asks[i].1;
            }
        }

        if sell_remaining == 0 {
            j += 1;
            if j < sell_bids.len() {
                sell_remaining = sell_bids[j].1;
            }
        }
    }

    // Subtract fixed withdrawal fee from profit
    total_profit -= fees.withdrawal_fee as i128;

    // Calculate average prices
    let (avg_buy, avg_sell) = if total_amount > 0 {
        let total_amount_scaled = total_amount as u128 / FixedPoint::SCALE as u128;
        if total_amount_scaled > 0 {
            (
                (total_buy_cost * FixedPoint::SCALE as u128 / total_amount_scaled) as u64,
                (total_sell_revenue * FixedPoint::SCALE as u128 / total_amount_scaled) as u64,
            )
        } else {
            (buy_asks[0].0, sell_bids[0].0)
        }
    } else {
        (0, 0)
    };

    OptimalSizeResult {
        amount: total_amount,
        profit: total_profit as i64,
        avg_buy_price: avg_buy,
        avg_sell_price: avg_sell,
        levels_consumed_buy: i + if buy_remaining < buy_asks.get(i).map(|x| x.1).unwrap_or(0) {
            1
        } else {
            0
        },
        levels_consumed_sell: j + if sell_remaining < sell_bids.get(j).map(|x| x.1).unwrap_or(0) {
            1
        } else {
            0
        },
    }
}

/// Calculate optimal size with f64 inputs for convenience.
pub fn calculate_optimal_size_f64(
    buy_asks: &[(f64, f64)],
    sell_bids: &[(f64, f64)],
    buy_fee_bps: u32,
    sell_fee_bps: u32,
    withdrawal_fee: f64,
) -> OptimalSizeResult {
    let buy_asks_u64: Vec<(u64, u64)> = buy_asks
        .iter()
        .map(|(p, q)| (FixedPoint::from_f64(*p).0, FixedPoint::from_f64(*q).0))
        .collect();

    let sell_bids_u64: Vec<(u64, u64)> = sell_bids
        .iter()
        .map(|(p, q)| (FixedPoint::from_f64(*p).0, FixedPoint::from_f64(*q).0))
        .collect();

    let fees = DepthFeeConfig {
        buy_fee_bps,
        sell_fee_bps,
        withdrawal_fee: FixedPoint::from_f64(withdrawal_fee).0,
    };

    calculate_optimal_size(&buy_asks_u64, &sell_bids_u64, fees)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(v: f64) -> u64 {
        FixedPoint::from_f64(v).0
    }

    #[test]
    fn test_simple_profitable_opportunity() {
        // Buy at 100, sell at 102 (2% spread)
        let buy_asks = vec![(fp(100.0), fp(10.0))]; // 10 units at 100
        let sell_bids = vec![(fp(102.0), fp(10.0))]; // 10 units at 102

        let fees = DepthFeeConfig {
            buy_fee_bps: 10,  // 0.1%
            sell_fee_bps: 10, // 0.1%
            withdrawal_fee: 0,
        };

        let result = calculate_optimal_size(&buy_asks, &sell_bids, fees);

        assert!(result.is_profitable());
        assert_eq!(result.amount, fp(10.0));
        // Profit = 10 * (102 * 0.999 - 100 * 1.001) = 10 * (101.898 - 100.1) = 17.98
        assert!(result.profit > 0);
    }

    #[test]
    fn test_no_profitable_opportunity() {
        // Buy at 100, sell at 100.1 (0.1% spread, less than fees)
        let buy_asks = vec![(fp(100.0), fp(10.0))];
        let sell_bids = vec![(fp(100.1), fp(10.0))];

        let fees = DepthFeeConfig {
            buy_fee_bps: 10,  // 0.1%
            sell_fee_bps: 10, // 0.1%
            withdrawal_fee: 0,
        };

        let result = calculate_optimal_size(&buy_asks, &sell_bids, fees);

        // After fees: buy at 100.1, sell at 99.9999 -> not profitable
        assert!(!result.is_profitable() || result.amount == 0);
    }

    #[test]
    fn test_multi_level_depth() {
        // Multiple levels on buy side
        let buy_asks = vec![
            (fp(100.0), fp(5.0)), // 5 units at 100
            (fp(100.5), fp(5.0)), // 5 units at 100.5
            (fp(101.0), fp(5.0)), // 5 units at 101 (will not be profitable)
        ];

        // Single level on sell side
        let sell_bids = vec![(fp(101.5), fp(15.0))]; // 15 units at 101.5

        let fees = DepthFeeConfig {
            buy_fee_bps: 10,
            sell_fee_bps: 10,
            withdrawal_fee: 0,
        };

        let result = calculate_optimal_size(&buy_asks, &sell_bids, fees);

        // Should consume first two buy levels (10 units total)
        // Third level at 101 with fees = 101.101, sell at 101.5 * 0.999 = 101.3985
        // Still profitable for level 3
        assert!(result.is_profitable());
        assert!(result.amount >= fp(10.0));
    }

    #[test]
    fn test_withdrawal_fee_impact() {
        let buy_asks = vec![(fp(100.0), fp(1.0))];
        let sell_bids = vec![(fp(102.0), fp(1.0))];

        // With high withdrawal fee
        let fees = DepthFeeConfig {
            buy_fee_bps: 10,
            sell_fee_bps: 10,
            withdrawal_fee: fp(100.0), // Very high fee
        };

        let result = calculate_optimal_size(&buy_asks, &sell_bids, fees);

        // Profit should be reduced by withdrawal fee
        // Gross profit ~ 1.8, minus 100 withdrawal fee = -98.2
        assert!(!result.is_profitable());
    }

    #[test]
    fn test_empty_orderbooks() {
        let result = calculate_optimal_size(&[], &[], DepthFeeConfig::default());
        assert_eq!(result.amount, 0);
        assert_eq!(result.profit, 0);

        let result =
            calculate_optimal_size(&[(fp(100.0), fp(1.0))], &[], DepthFeeConfig::default());
        assert_eq!(result.amount, 0);
    }

    #[test]
    fn test_f64_interface() {
        let buy_asks = vec![(100.0, 10.0)];
        let sell_bids = vec![(102.0, 10.0)];

        let result = calculate_optimal_size_f64(&buy_asks, &sell_bids, 10, 10, 0.0);

        assert!(result.is_profitable());
    }

    #[test]
    fn test_partial_fill() {
        // More demand on sell side than supply on buy side
        let buy_asks = vec![(fp(100.0), fp(5.0))]; // Only 5 units available
        let sell_bids = vec![(fp(102.0), fp(10.0))]; // 10 units wanted

        let fees = DepthFeeConfig::default();
        let result = calculate_optimal_size(&buy_asks, &sell_bids, fees);

        // Should only fill 5 units (limited by buy side)
        assert_eq!(result.amount, fp(5.0));
    }

    #[test]
    fn test_crossing_levels() {
        // Asymmetric depth - should walk through multiple levels on both sides
        let buy_asks = vec![
            (fp(100.0), fp(3.0)),
            (fp(100.2), fp(3.0)),
            (fp(100.4), fp(3.0)),
        ];

        let sell_bids = vec![
            (fp(102.0), fp(2.0)),
            (fp(101.8), fp(2.0)),
            (fp(101.6), fp(2.0)),
        ];

        let fees = DepthFeeConfig {
            buy_fee_bps: 10,
            sell_fee_bps: 10,
            withdrawal_fee: 0,
        };

        let result = calculate_optimal_size(&buy_asks, &sell_bids, fees);

        // Should consume multiple levels on both sides
        assert!(result.is_profitable());
        assert!(result.levels_consumed_buy >= 1);
        assert!(result.levels_consumed_sell >= 1);
    }
}
