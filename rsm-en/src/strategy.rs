use crate::market::{MarketBar, OrderSide};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: String,
    pub quantity: f64,
    pub entry_price: f64,
    pub side: OrderSide,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Signal {
    Buy,
    Sell,
    Hold,
}
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
pub trait Strategy: Send + Sync {
    fn name(&self) -> &str;
    fn generate_signal(&mut self, context: &StrategyContext) -> Signal;
    fn reset(&mut self);
}
