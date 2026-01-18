//! Execution configuration and state types.

use serde::{Deserialize, Serialize};

/// Execution mode for arbitrage opportunities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum ExecutionMode {
    /// Automatically execute when conditions are met
    Auto = 0,
    /// Require manual approval before execution
    ManualApproval = 1,
    /// Only send alerts, no execution
    AlertOnly = 2,
}

impl ExecutionMode {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(ExecutionMode::Auto),
            1 => Some(ExecutionMode::ManualApproval),
            2 => Some(ExecutionMode::AlertOnly),
            _ => None,
        }
    }

    pub fn requires_approval(self) -> bool {
        matches!(self, ExecutionMode::ManualApproval)
    }
}

/// Configuration for trade execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionConfig {
    pub mode: ExecutionMode,
    /// Maximum position size in USD (fixed-point 8 decimals)
    pub max_position_usd: u64,
    /// Maximum allowed slippage in basis points
    pub max_slippage_bps: u16,
    /// Minimum profit required in basis points
    pub min_profit_bps: i32,
    /// Auto-execute if position is below this USD amount
    pub auto_execute_below_usd: u64,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::ManualApproval,
            max_position_usd: 10000_00000000,     // $10,000
            max_slippage_bps: 50,                 // 0.5%
            min_profit_bps: 30,                   // 0.3%
            auto_execute_below_usd: 100_00000000, // $100
        }
    }
}

impl ExecutionConfig {
    /// Check if this trade should auto-execute.
    pub fn should_auto_execute(&self, position_usd: u64, profit_bps: i32) -> bool {
        if self.mode != ExecutionMode::Auto {
            return false;
        }
        if position_usd > self.auto_execute_below_usd {
            return false;
        }
        if profit_bps < self.min_profit_bps {
            return false;
        }
        true
    }
}

/// Order execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum OrderStatus {
    Pending = 0,
    Submitted = 1,
    PartiallyFilled = 2,
    Filled = 3,
    Cancelled = 4,
    Failed = 5,
}

impl OrderStatus {
    /// Check if this is a terminal state.
    pub fn is_final(self) -> bool {
        matches!(
            self,
            OrderStatus::Filled | OrderStatus::Cancelled | OrderStatus::Failed
        )
    }

    /// Check if order is currently active.
    pub fn is_active(self) -> bool {
        matches!(self, OrderStatus::Submitted | OrderStatus::PartiallyFilled)
    }
}

/// Record of an executed step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedStep {
    pub step_index: u8,
    pub description: String,
    pub timestamp_ms: u64,
    pub tx_hash: Option<String>,
}

/// Current state of an execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    pub opportunity_id: u64,
    pub status: OrderStatus,
    pub executed_steps: Vec<ExecutedStep>,
    pub current_step: u8,
    /// Realized profit/loss
    pub realized_pnl: i64,
    pub error_message: Option<String>,
    pub started_at_ms: u64,
    pub completed_at_ms: Option<u64>,
}

impl ExecutionState {
    pub fn new(opportunity_id: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            opportunity_id,
            status: OrderStatus::Pending,
            executed_steps: Vec::new(),
            current_step: 0,
            realized_pnl: 0,
            error_message: None,
            started_at_ms: now,
            completed_at_ms: None,
        }
    }

    /// Mark a step as complete.
    pub fn complete_step(&mut self, description: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.executed_steps.push(ExecutedStep {
            step_index: self.current_step,
            description: description.to_string(),
            timestamp_ms: now,
            tx_hash: None,
        });
        self.current_step += 1;
    }

    /// Mark execution as failed.
    pub fn fail(&mut self, message: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        self.status = OrderStatus::Failed;
        self.error_message = Some(message.to_string());
        self.completed_at_ms = Some(now);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === ExecutionMode tests ===

    #[test]
    fn test_execution_mode_from_id() {
        assert_eq!(ExecutionMode::from_id(0), Some(ExecutionMode::Auto));
        assert_eq!(
            ExecutionMode::from_id(1),
            Some(ExecutionMode::ManualApproval)
        );
        assert_eq!(ExecutionMode::from_id(2), Some(ExecutionMode::AlertOnly));
        assert_eq!(ExecutionMode::from_id(255), None);
    }

    #[test]
    fn test_execution_mode_requires_approval() {
        assert!(!ExecutionMode::Auto.requires_approval());
        assert!(ExecutionMode::ManualApproval.requires_approval());
        assert!(!ExecutionMode::AlertOnly.requires_approval());
    }

    // === ExecutionConfig tests ===

    #[test]
    fn test_execution_config_default() {
        let config = ExecutionConfig::default();

        assert_eq!(config.mode, ExecutionMode::ManualApproval);
        assert!(config.max_position_usd > 0);
        assert!(config.min_profit_bps > 0);
    }

    #[test]
    fn test_execution_config_should_auto_execute() {
        let mut config = ExecutionConfig::default();
        config.mode = ExecutionMode::Auto;
        config.auto_execute_below_usd = 100_00000000; // $100
        config.min_profit_bps = 50; // 0.5%

        // Below threshold, should auto execute
        assert!(config.should_auto_execute(50_00000000, 100)); // $50, 1% profit

        // Above threshold
        assert!(!config.should_auto_execute(200_00000000, 100)); // $200

        // Below min profit
        assert!(!config.should_auto_execute(50_00000000, 10)); // $50, 0.1% profit

        // Manual mode
        config.mode = ExecutionMode::ManualApproval;
        assert!(!config.should_auto_execute(50_00000000, 100));
    }

    // === OrderStatus tests ===

    #[test]
    fn test_order_status_is_final() {
        assert!(!OrderStatus::Pending.is_final());
        assert!(!OrderStatus::Submitted.is_final());
        assert!(!OrderStatus::PartiallyFilled.is_final());
        assert!(OrderStatus::Filled.is_final());
        assert!(OrderStatus::Cancelled.is_final());
        assert!(OrderStatus::Failed.is_final());
    }

    #[test]
    fn test_order_status_is_active() {
        assert!(!OrderStatus::Pending.is_active());
        assert!(OrderStatus::Submitted.is_active());
        assert!(OrderStatus::PartiallyFilled.is_active());
        assert!(!OrderStatus::Filled.is_active());
    }

    // === ExecutionState tests ===

    #[test]
    fn test_execution_state_new() {
        let state = ExecutionState::new(12345);

        assert_eq!(state.opportunity_id, 12345);
        assert_eq!(state.status, OrderStatus::Pending);
        assert_eq!(state.current_step, 0);
        assert_eq!(state.realized_pnl, 0);
    }

    #[test]
    fn test_execution_state_advance_step() {
        let mut state = ExecutionState::new(1);
        state.status = OrderStatus::Submitted;

        state.complete_step("Step 1 done");
        assert_eq!(state.current_step, 1);
        assert_eq!(state.executed_steps.len(), 1);
        assert_eq!(state.executed_steps[0].description, "Step 1 done");

        state.complete_step("Step 2 done");
        assert_eq!(state.current_step, 2);
        assert_eq!(state.executed_steps.len(), 2);
    }

    #[test]
    fn test_execution_state_fail() {
        let mut state = ExecutionState::new(1);
        state.status = OrderStatus::Submitted;

        state.fail("Network error");
        assert_eq!(state.status, OrderStatus::Failed);
        assert_eq!(state.error_message, Some("Network error".to_string()));
    }
}
