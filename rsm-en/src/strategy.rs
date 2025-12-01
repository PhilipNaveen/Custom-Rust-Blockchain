use crate::market::{MarketBar, OrderSide};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Position in the market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub side: OrderSide,
}

/// Trading signal
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Signal {
    Buy,
    Sell,
    Hold,
}

/// Strategy context with market data
pub struct StrategyContext<'a> {
    pub bars: &'a [MarketBar],
    pub current_index: usize,
    pub position: Option<&'a Position>,
}

impl<'a> StrategyContext<'a> {
    pub fn current_bar(&self) -> &MarketBar {
        &self.bars[self.current_index]
    }

    pub fn lookback(&self, periods: usize) -> &[MarketBar] {
        let start = self.current_index.saturating_sub(periods);
        &self.bars[start..=self.current_index]
    }
}

/// Base trait for trading strategies
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    fn generate_signal(&mut self, context: &StrategyContext) -> Signal;
    fn reset(&mut self);
}

/// Simple Moving Average Crossover Strategy
#[derive(Debug, Clone)]
pub struct MovingAverageCrossover {
    pub fast_period: usize,
    pub slow_period: usize,
    pub last_signal: Signal,
}

impl MovingAverageCrossover {
    pub fn new(fast_period: usize, slow_period: usize) -> Self {
        Self {
            fast_period,
            slow_period,
            last_signal: Signal::Hold,
        }
    }

    fn calculate_sma(bars: &[MarketBar], period: usize) -> Option<f64> {
        if bars.len() < period {
            return None;
        }
        
        let sum: f64 = bars.iter().rev().take(period).map(|b| b.close).sum();
        Some(sum / period as f64)
    }
}

impl Strategy for MovingAverageCrossover {
    fn name(&self) -> &str {
        "MA Crossover"
    }

    fn generate_signal(&mut self, context: &StrategyContext) -> Signal {
        let bars = context.lookback(self.slow_period);
        
        if bars.len() < self.slow_period {
            return Signal::Hold;
        }

        let fast_ma = Self::calculate_sma(bars, self.fast_period);
        let slow_ma = Self::calculate_sma(bars, self.slow_period);

        match (fast_ma, slow_ma) {
            (Some(fast), Some(slow)) => {
                let signal = if fast > slow && self.last_signal != Signal::Buy {
                    Signal::Buy
                } else if fast < slow && self.last_signal != Signal::Sell {
                    Signal::Sell
                } else {
                    Signal::Hold
                };
                
                if signal != Signal::Hold {
                    self.last_signal = signal;
                }
                
                signal
            }
            _ => Signal::Hold,
        }
    }

    fn reset(&mut self) {
        self.last_signal = Signal::Hold;
    }
}

/// Mean Reversion Strategy
#[derive(Debug, Clone)]
pub struct MeanReversion {
    pub lookback_period: usize,
    pub entry_threshold: f64,
    pub exit_threshold: f64,
}

impl MeanReversion {
    pub fn new(lookback_period: usize, entry_threshold: f64, exit_threshold: f64) -> Self {
        Self {
            lookback_period,
            entry_threshold,
            exit_threshold,
        }
    }

    fn calculate_z_score(bars: &[MarketBar], period: usize) -> Option<f64> {
        if bars.len() < period {
            return None;
        }

        let prices: Vec<f64> = bars.iter().rev().take(period).map(|b| b.close).collect();
        let mean = prices.iter().sum::<f64>() / period as f64;
        
        let variance = prices.iter()
            .map(|p| (p - mean).powi(2))
            .sum::<f64>() / period as f64;
        
        let std_dev = variance.sqrt();
        
        if std_dev == 0.0 {
            return Some(0.0);
        }

        let current_price = bars.last().unwrap().close;
        Some((current_price - mean) / std_dev)
    }
}

impl Strategy for MeanReversion {
    fn name(&self) -> &str {
        "Mean Reversion"
    }

    fn generate_signal(&mut self, context: &StrategyContext) -> Signal {
        let bars = context.lookback(self.lookback_period);
        
        if let Some(z_score) = Self::calculate_z_score(bars, self.lookback_period) {
            if context.position.is_some() {
                // Exit logic
                if z_score.abs() < self.exit_threshold {
                    return Signal::Sell; // Close position
                }
            } else {
                // Entry logic
                if z_score < -self.entry_threshold {
                    return Signal::Buy; // Buy when oversold
                } else if z_score > self.entry_threshold {
                    return Signal::Sell; // Short when overbought (or stay out)
                }
            }
        }

        Signal::Hold
    }

    fn reset(&mut self) {
        // No state to reset
    }
}

/// Momentum Strategy
#[derive(Debug, Clone)]
pub struct Momentum {
    pub lookback_period: usize,
    pub threshold: f64,
    pub prices: VecDeque<f64>,
}

impl Momentum {
    pub fn new(lookback_period: usize, threshold: f64) -> Self {
        Self {
            lookback_period,
            threshold,
            prices: VecDeque::new(),
        }
    }

    fn calculate_momentum(&self) -> Option<f64> {
        if self.prices.len() < self.lookback_period {
            return None;
        }

        let current = *self.prices.back().unwrap();
        let past = *self.prices.front().unwrap();
        
        if past == 0.0 {
            return Some(0.0);
        }

        Some((current - past) / past)
    }
}

impl Strategy for Momentum {
    fn name(&self) -> &str {
        "Momentum"
    }

    fn generate_signal(&mut self, context: &StrategyContext) -> Signal {
        let current_bar = context.current_bar();
        
        self.prices.push_back(current_bar.close);
        if self.prices.len() > self.lookback_period {
            self.prices.pop_front();
        }

        if let Some(momentum) = self.calculate_momentum() {
            if momentum > self.threshold {
                return Signal::Buy;
            } else if momentum < -self.threshold {
                return Signal::Sell;
            }
        }

        Signal::Hold
    }

    fn reset(&mut self) {
        self.prices.clear();
    }
}

/// RSI (Relative Strength Index) Strategy
#[derive(Debug, Clone)]
pub struct RSIStrategy {
    pub period: usize,
    pub oversold: f64,
    pub overbought: f64,
    pub prices: VecDeque<f64>,
}

impl RSIStrategy {
    pub fn new(period: usize, oversold: f64, overbought: f64) -> Self {
        Self {
            period,
            oversold,
            overbought,
            prices: VecDeque::new(),
        }
    }

    fn calculate_rsi(&self) -> Option<f64> {
        if self.prices.len() < self.period + 1 {
            return None;
        }

        let prices: Vec<f64> = self.prices.iter().copied().collect();
        let mut gains = 0.0;
        let mut losses = 0.0;

        for i in 1..=self.period {
            let change = prices[i] - prices[i - 1];
            if change > 0.0 {
                gains += change;
            } else {
                losses += change.abs();
            }
        }

        let avg_gain = gains / self.period as f64;
        let avg_loss = losses / self.period as f64;

        if avg_loss == 0.0 {
            return Some(100.0);
        }

        let rs = avg_gain / avg_loss;
        Some(100.0 - (100.0 / (1.0 + rs)))
    }
}

impl Strategy for RSIStrategy {
    fn name(&self) -> &str {
        "RSI"
    }

    fn generate_signal(&mut self, context: &StrategyContext) -> Signal {
        let current_bar = context.current_bar();
        
        self.prices.push_back(current_bar.close);
        if self.prices.len() > self.period + 1 {
            self.prices.pop_front();
        }

        if let Some(rsi) = self.calculate_rsi() {
            if context.position.is_none() {
                if rsi < self.oversold {
                    return Signal::Buy;
                } else if rsi > self.overbought {
                    return Signal::Sell;
                }
            } else {
                // Exit when RSI returns to neutral
                if rsi > 40.0 && rsi < 60.0 {
                    return Signal::Sell; // Close position
                }
            }
        }

        Signal::Hold
    }

    fn reset(&mut self) {
        self.prices.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ma_crossover_creation() {
        let strategy = MovingAverageCrossover::new(10, 20);
        assert_eq!(strategy.fast_period, 10);
        assert_eq!(strategy.slow_period, 20);
    }

    #[test]
    fn test_mean_reversion_creation() {
        let strategy = MeanReversion::new(20, 2.0, 0.5);
        assert_eq!(strategy.lookback_period, 20);
    }

    #[test]
    fn test_momentum_creation() {
        let strategy = Momentum::new(10, 0.02);
        assert_eq!(strategy.lookback_period, 10);
    }
}
