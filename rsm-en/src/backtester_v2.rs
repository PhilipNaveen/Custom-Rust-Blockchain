use crate::market::{MarketBar, OrderSide};
use crate::strategy::{Position, Signal, Strategy, StrategyContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Debug, Clone)]
pub enum ExecutionModel {
    Naive,
    Realistic,
    Conservative,
}
#[derive(Debug, Clone)]
pub struct TransactionCosts {
    pub commission_rate: f64,
    pub slippage_bps: f64,
    pub market_impact_factor: f64,
}

impl Default for TransactionCosts {
    fn default() -> Self {
        Self {
            commission_rate: 0.001,
            slippage_bps: 5.0,
            market_impact_factor: 0.1,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecution {
    pub timestamp: u64,
    pub symbol: String,
    pub side: OrderSide,
    pub intended_price: f64,
    pub fill_price: f64,
    pub quantity: f64,
    pub commission: f64,
    pub slippage: f64,
    pub market_impact: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSnapshot {
    pub timestamp: u64,
    pub cash: f64,
    pub positions_value: f64,
    pub total_value: f64,
    pub positions: Vec<Position>,
    pub leverage: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub total_return: f64,
    pub annualized_return: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
    pub calmar_ratio: f64,
    pub max_drawdown: f64,
    pub avg_drawdown: f64,
    pub win_rate: f64,
    pub profit_factor: f64,
    pub avg_win: f64,
    pub avg_loss: f64,
    pub largest_win: f64,
    pub largest_loss: f64,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub total_commission: f64,
    pub total_slippage: f64,
    pub avg_holding_period: f64,
    pub return_volatility: f64,
}
pub struct Backtester {
    pub initial_capital: f64,
    pub cash: f64,
    pub positions: HashMap<String, Position>,
    pub trade_history: Vec<TradeExecution>,
    pub portfolio_history: Vec<PortfolioSnapshot>,
    pub costs: TransactionCosts,
    pub execution_model: ExecutionModel,
    

    pub max_position_size: f64,
    pub max_leverage: f64,
    pub use_stops: bool,
    pub stop_loss_pct: f64,
}

impl Backtester {
    pub fn new(initial_capital: f64) -> Self {
        Self {
            initial_capital,
            cash: initial_capital,
            positions: HashMap::new(),
            trade_history: Vec::new(),
            portfolio_history: Vec::new(),
            costs: TransactionCosts::default(),
            execution_model: ExecutionModel::Realistic,
            max_position_size: 1.0,
            max_leverage: 1.0,
            use_stops: false,
            stop_loss_pct: 0.05,
        }
    }
    
    pub fn with_costs(mut self, costs: TransactionCosts) -> Self {
        self.costs = costs;
        self
    }
    
    pub fn with_execution_model(mut self, model: ExecutionModel) -> Self {
        self.execution_model = model;
        self
    }
    
    pub fn with_risk_controls(mut self, max_position_size: f64, stop_loss_pct: f64) -> Self {
        self.max_position_size = max_position_size;
        self.stop_loss_pct = stop_loss_pct;
        self.use_stops = true;
        self
    }
    fn calculate_fill_price(&self, intended_price: f64, side: OrderSide, quantity: f64, volume: f64) -> (f64, f64, f64) {
        match self.execution_model {
            ExecutionModel::Naive => (intended_price, 0.0, 0.0),
            ExecutionModel::Realistic | ExecutionModel::Conservative => {

                let slippage_amount = intended_price * (self.costs.slippage_bps / 10000.0);
                

                let participation_rate = if volume > 0.0 {
                    (quantity / volume).min(1.0)
                } else {
                    0.1
                };
                let market_impact = intended_price * self.costs.market_impact_factor * 
                    (participation_rate.sqrt() / 100.0);
                
                let total_impact = slippage_amount + market_impact;
                
                let fill_price = match side {
                    OrderSide::Buy => intended_price + total_impact,
                    OrderSide::Sell => intended_price - total_impact,
                };
                

                let adjustment = if matches!(self.execution_model, ExecutionModel::Conservative) {
                    1.5
                } else {
                    1.0
                };
                
                (fill_price, slippage_amount * adjustment, market_impact * adjustment)
            }
        }
    }

    pub fn execute_trade(
        &mut self, 
        timestamp: u64, 
        symbol: String, 
        side: OrderSide, 
        intended_price: f64, 
        quantity: f64,
        volume: f64,
    ) -> Result<(), String> {
        let (fill_price, slippage, market_impact) = 
            self.calculate_fill_price(intended_price, side, quantity, volume);
        
        let trade_value = fill_price * quantity;
        let commission = trade_value * self.costs.commission_rate;

        match side {
            OrderSide::Buy => {
                let total_cost = trade_value + commission;
                if self.cash < total_cost {
                    return Err("Insufficient cash for trade".to_string());
                }

                self.cash -= total_cost;
                self.positions.entry(symbol.clone())
                    .and_modify(|pos| {
                        let total_quantity = pos.quantity + quantity;
                        let total_cost = (pos.entry_price * pos.quantity) + (fill_price * quantity);
                        pos.entry_price = total_cost / total_quantity;
                        pos.quantity = total_quantity;
                    })
                    .or_insert(Position {
                        symbol: symbol.clone(),
                        quantity,
                        entry_price: fill_price,
                        side: OrderSide::Buy,
                    });

                self.trade_history.push(TradeExecution {
                    timestamp,
                    symbol,
                    side: OrderSide::Buy,
                    intended_price,
                    fill_price,
                    quantity,
                    commission,
                    slippage,
                    market_impact,
                });

                Ok(())
            }
            OrderSide::Sell => {
                if let Some(position) = self.positions.get_mut(&symbol) {
                    if position.quantity < quantity {
                        return Err("Insufficient position for trade".to_string());
                    }

                    let proceeds = trade_value - commission;
                    self.cash += proceeds;

                    position.quantity -= quantity;

                    if position.quantity <= 0.0001 {
                        self.positions.remove(&symbol);
                    }

                    self.trade_history.push(TradeExecution {
                        timestamp,
                        symbol,
                        side: OrderSide::Sell,
                        intended_price,
                        fill_price,
                        quantity,
                        commission,
                        slippage,
                        market_impact,
                    });

                    Ok(())
                } else {
                    Err("No position to sell".to_string())
                }
            }
        }
    }

    pub fn calculate_portfolio_value(&self, prices: &HashMap<String, f64>) -> f64 {
        let positions_value: f64 = self.positions.iter()
            .map(|(symbol, position)| {
                let price = prices.get(symbol).unwrap_or(&position.entry_price);
                price * position.quantity
            })
            .sum();

        self.cash + positions_value
    }

    pub fn record_snapshot(&mut self, timestamp: u64, prices: &HashMap<String, f64>) {
        let positions_value: f64 = self.positions.iter()
            .map(|(symbol, position)| {
                let price = prices.get(symbol).unwrap_or(&position.entry_price);
                price * position.quantity
            })
            .sum();

        let total_value = self.cash + positions_value;
        let leverage = if self.cash > 0.0 {
            positions_value / self.cash
        } else {
            0.0
        };

        self.portfolio_history.push(PortfolioSnapshot {
            timestamp,
            cash: self.cash,
            positions_value,
            total_value,
            positions: self.positions.values().cloned().collect(),
            leverage,
        });
    }
    

    fn check_stop_loss(&self, position: &Position, current_price: f64) -> bool {
        if !self.use_stops {
            return false;
        }
        
        let pnl_pct = (current_price - position.entry_price) / position.entry_price;
        pnl_pct < -self.stop_loss_pct
    }

    pub fn run_backtest<S: Strategy>(
        &mut self,
        strategy: &mut S,
        bars: &[MarketBar],
        position_size: f64,
    ) -> BacktestResult {
        strategy.reset();
        
        let symbol = bars[0].symbol.clone();
        
        for (idx, bar) in bars.iter().enumerate() {
            let position = self.positions.get(&symbol);
            

            if let Some(pos) = position {
                if self.check_stop_loss(pos, bar.close) {
                    let _ = self.execute_trade(
                        bar.timestamp,
                        symbol.clone(),
                        OrderSide::Sell,
                        bar.close,
                        pos.quantity,
                        bar.volume,
                    );
                }
            }
            
            let position = self.positions.get(&symbol);
            let context = StrategyContext {
                bars,
                current_index: idx,
                position,
            };

            let signal = strategy.generate_signal(&context);

            match signal {
                Signal::Buy => {
                    if position.is_none() {
                        let max_buy_value = self.cash * position_size.min(self.max_position_size);
                        let quantity = max_buy_value / bar.close;
                        if quantity > 0.0 {
                            let _ = self.execute_trade(
                                bar.timestamp,
                                symbol.clone(),
                                OrderSide::Buy,
                                bar.close,
                                quantity,
                                bar.volume,
                            );
                        }
                    }
                }
                Signal::Sell => {
                    if let Some(pos) = position {
                        let _ = self.execute_trade(
                            bar.timestamp,
                            symbol.clone(),
                            OrderSide::Sell,
                            bar.close,
                            pos.quantity,
                            bar.volume,
                        );
                    }
                }
                Signal::Hold => {}
            }

            let mut prices = HashMap::new();
            prices.insert(symbol.clone(), bar.close);
            self.record_snapshot(bar.timestamp, &prices);
        }
        if let Some(last_bar) = bars.last() {
            let symbol = last_bar.symbol.clone();
            if let Some(position) = self.positions.get(&symbol).cloned() {
                let _ = self.execute_trade(
                    last_bar.timestamp,
                    symbol,
                    OrderSide::Sell,
                    last_bar.close,
                    position.quantity,
                    last_bar.volume,
                );
            }
        }

        self.calculate_backtest_result(strategy.name(), bars.len())
    }

    fn calculate_backtest_result(&self, strategy_name: &str, num_bars: usize) -> BacktestResult {
        let metrics = self.calculate_advanced_metrics(num_bars);
        
        BacktestResult {
            strategy_name: strategy_name.to_string(),
            initial_capital: self.initial_capital,
            metrics,
            portfolio_history: self.portfolio_history.clone(),
            trade_history: self.trade_history.clone(),
        }
    }
    
    fn calculate_advanced_metrics(&self, num_bars: usize) -> PerformanceMetrics {
        let final_value = self.portfolio_history.last()
            .map(|s| s.total_value)
            .unwrap_or(self.initial_capital);

        let total_return = (final_value - self.initial_capital) / self.initial_capital;
        

        let returns: Vec<f64> = self.portfolio_history.windows(2)
            .map(|w| {
                if w[0].total_value == 0.0 {
                    0.0
                } else {
                    (w[1].total_value - w[0].total_value) / w[0].total_value
                }
            })
            .collect();
        let sharpe_ratio = calculate_sharpe_ratio(&returns);
        

        let sortino_ratio = calculate_sortino_ratio(&returns);
        

        let (max_drawdown, avg_drawdown) = calculate_drawdowns(&self.portfolio_history);
        

        let annualized_return = total_return * (252.0 / num_bars as f64);
        let calmar_ratio = if max_drawdown > 0.0 {
            annualized_return / max_drawdown
        } else {
            0.0
        };
        

        let (win_rate, profit_factor, avg_win, avg_loss, largest_win, largest_loss, 
             winning_trades, losing_trades) = calculate_trade_stats(&self.trade_history);
        
        let total_commission: f64 = self.trade_history.iter().map(|t| t.commission).sum();
        let total_slippage: f64 = self.trade_history.iter()
            .map(|t| t.slippage + t.market_impact)
            .sum();
        

        let avg_holding_period = calculate_avg_holding_period(&self.trade_history);
        

        let return_volatility = if returns.len() > 1 {
            let mean = returns.iter().sum::<f64>() / returns.len() as f64;
            let variance: f64 = returns.iter()
                .map(|r| (r - mean).powi(2))
                .sum::<f64>() / returns.len() as f64;
            variance.sqrt() * (252.0_f64).sqrt()
        } else {
            0.0
        };
        
        PerformanceMetrics {
            total_return,
            annualized_return,
            sharpe_ratio,
            sortino_ratio,
            calmar_ratio,
            max_drawdown,
            avg_drawdown,
            win_rate,
            profit_factor,
            avg_win,
            avg_loss,
            largest_win,
            largest_loss,
            total_trades: self.trade_history.len() / 2,
            winning_trades,
            losing_trades,
            total_commission,
            total_slippage,
            avg_holding_period,
            return_volatility,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacktestResult {
    pub strategy_name: String,
    pub initial_capital: f64,
    pub metrics: PerformanceMetrics,
    pub portfolio_history: Vec<PortfolioSnapshot>,
    pub trade_history: Vec<TradeExecution>,
}

impl BacktestResult {
    pub fn print_summary(&self) {
        let m = &self.metrics;
        println!("\n{}", "=".repeat(70));
        println!("Backtest Results: {}", self.strategy_name);
        println!("{}", "=".repeat(70));
        println!("Initial Capital:     ${:.2}", self.initial_capital);
        println!("Final Value:         ${:.2}", 
            self.portfolio_history.last().map(|s| s.total_value).unwrap_or(0.0));
        println!("Total Return:        {:.2}%", m.total_return * 100.0);
        println!("Annualized Return:   {:.2}%", m.annualized_return * 100.0);
        println!("Return Volatility:   {:.2}%", m.return_volatility * 100.0);
        println!();
        println!("Risk-Adjusted Returns:");
        println!("  Sharpe Ratio:      {:.4}", m.sharpe_ratio);
        println!("  Sortino Ratio:     {:.4}", m.sortino_ratio);
        println!("  Calmar Ratio:      {:.4}", m.calmar_ratio);
        println!();
        println!("Drawdown Analysis:");
        println!("  Max Drawdown:      {:.2}%", m.max_drawdown * 100.0);
        println!("  Avg Drawdown:      {:.2}%", m.avg_drawdown * 100.0);
        println!();
        println!("Trade Statistics:");
        println!("  Total Trades:      {}", m.total_trades);
        println!("  Win Rate:          {:.2}%", m.win_rate * 100.0);
        println!("  Profit Factor:     {:.4}", m.profit_factor);
        println!("  Avg Win:           ${:.2}", m.avg_win);
        println!("  Avg Loss:          ${:.2}", m.avg_loss);
        println!("  Largest Win:       ${:.2}", m.largest_win);
        println!("  Largest Loss:      ${:.2}", m.largest_loss);
        println!("  Avg Holding:       {:.1} bars", m.avg_holding_period);
        println!();
        println!("Transaction Costs:");
        println!("  Total Commission:  ${:.2}", m.total_commission);
        println!("  Total Slippage:    ${:.2}", m.total_slippage);
        println!("  Total Costs:       ${:.2}", m.total_commission + m.total_slippage);
        println!("{}", "=".repeat(70));
    }

    pub fn get_equity_curve(&self) -> Vec<(u64, f64)> {
        self.portfolio_history.iter()
            .map(|snapshot| (snapshot.timestamp, snapshot.total_value))
            .collect()
    }
}

fn calculate_sharpe_ratio(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }

    let mean_return: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
    
    let variance: f64 = returns.iter()
        .map(|r| (r - mean_return).powi(2))
        .sum::<f64>() / returns.len() as f64;
    
    let std_dev = variance.sqrt();
    
    if std_dev == 0.0 {
        return 0.0;
    }

    let annualized_return = mean_return * 252.0;
    let annualized_std = std_dev * (252.0_f64).sqrt();
    
    annualized_return / annualized_std
}

fn calculate_sortino_ratio(returns: &[f64]) -> f64 {
    if returns.is_empty() {
        return 0.0;
    }

    let mean_return: f64 = returns.iter().sum::<f64>() / returns.len() as f64;
    

    let downside_returns: Vec<f64> = returns.iter()
        .filter(|&&r| r < 0.0)
        .copied()
        .collect();
    
    if downside_returns.is_empty() {
        return if mean_return > 0.0 { 100.0 } else { 0.0 };
    }
    
    let downside_variance: f64 = downside_returns.iter()
        .map(|r| r.powi(2))
        .sum::<f64>() / returns.len() as f64;
    
    let downside_std = downside_variance.sqrt();
    
    if downside_std == 0.0 {
        return 0.0;
    }

    let annualized_return = mean_return * 252.0;
    let annualized_downside_std = downside_std * (252.0_f64).sqrt();
    
    annualized_return / annualized_downside_std
}

fn calculate_drawdowns(portfolio_history: &[PortfolioSnapshot]) -> (f64, f64) {
    if portfolio_history.is_empty() {
        return (0.0, 0.0);
    }

    let mut max_value = portfolio_history[0].total_value;
    let mut max_drawdown = 0.0;
    let mut drawdowns = Vec::new();

    for snapshot in portfolio_history {
        if snapshot.total_value > max_value {
            max_value = snapshot.total_value;
        }

        let drawdown = (max_value - snapshot.total_value) / max_value;
        if drawdown > 0.0 {
            drawdowns.push(drawdown);
        }
        if drawdown > max_drawdown {
            max_drawdown = drawdown;
        }
    }

    let avg_drawdown = if !drawdowns.is_empty() {
        drawdowns.iter().sum::<f64>() / drawdowns.len() as f64
    } else {
        0.0
    };

    (max_drawdown, avg_drawdown)
}

fn calculate_trade_stats(trade_history: &[TradeExecution]) -> (f64, f64, f64, f64, f64, f64, usize, usize) {
    if trade_history.len() < 2 {
        return (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0, 0);
    }

    let mut wins = Vec::new();
    let mut losses = Vec::new();
    let mut i = 0;

    while i < trade_history.len() - 1 {
        if trade_history[i].side == OrderSide::Buy {
            if let Some(sell_idx) = trade_history[i+1..].iter()
                .position(|t| t.side == OrderSide::Sell && t.symbol == trade_history[i].symbol) {
                
                let buy_trade = &trade_history[i];
                let sell_trade = &trade_history[i + 1 + sell_idx];
                
                let profit = (sell_trade.fill_price - buy_trade.fill_price) * buy_trade.quantity
                    - buy_trade.commission - sell_trade.commission
                    - (buy_trade.slippage + buy_trade.market_impact) * buy_trade.quantity
                    - (sell_trade.slippage + sell_trade.market_impact) * sell_trade.quantity;
                
                if profit > 0.0 {
                    wins.push(profit);
                } else {
                    losses.push(profit.abs());
                }
                
                i = i + 1 + sell_idx + 1;
            } else {
                break;
            }
        } else {
            i += 1;
        }
    }

    let win_count = wins.len();
    let loss_count = losses.len();
    let total_trades = win_count + loss_count;
    
    let win_rate = if total_trades > 0 {
        win_count as f64 / total_trades as f64
    } else {
        0.0
    };
    
    let total_wins: f64 = wins.iter().sum();
    let total_losses: f64 = losses.iter().sum();
    
    let profit_factor = if total_losses > 0.0 {
        total_wins / total_losses
    } else if total_wins > 0.0 {
        100.0
    } else {
        0.0
    };
    
    let avg_win = if !wins.is_empty() {
        total_wins / wins.len() as f64
    } else {
        0.0
    };
    
    let avg_loss = if !losses.is_empty() {
        total_losses / losses.len() as f64
    } else {
        0.0
    };
    
    let largest_win = wins.iter().fold(0.0_f64, |a, &b| a.max(b));
    let largest_loss = losses.iter().fold(0.0_f64, |a, &b| a.max(b));
    
    (win_rate, profit_factor, avg_win, avg_loss, largest_win, largest_loss, win_count, loss_count)
}

fn calculate_avg_holding_period(trade_history: &[TradeExecution]) -> f64 {
    if trade_history.len() < 2 {
        return 0.0;
    }

    let mut holding_periods = Vec::new();
    let mut i = 0;

    while i < trade_history.len() - 1 {
        if trade_history[i].side == OrderSide::Buy {
            if let Some(sell_idx) = trade_history[i+1..].iter()
                .position(|t| t.side == OrderSide::Sell && t.symbol == trade_history[i].symbol) {
                
                let buy_time = trade_history[i].timestamp;
                let sell_time = trade_history[i + 1 + sell_idx].timestamp;
                holding_periods.push((sell_time - buy_time) as f64);
                
                i = i + 1 + sell_idx + 1;
            } else {
                break;
            }
        } else {
            i += 1;
        }
    }

    if holding_periods.is_empty() {
        0.0
    } else {
        holding_periods.iter().sum::<f64>() / holding_periods.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backtester_creation() {
        let backtester = Backtester::new(10000.0);
        assert_eq!(backtester.initial_capital, 10000.0);
        assert_eq!(backtester.cash, 10000.0);
    }

    #[test]
    fn test_sharpe_ratio() {
        let returns = vec![0.01, 0.02, -0.01, 0.015, 0.005];
        let sharpe = calculate_sharpe_ratio(&returns);
        assert!(sharpe > 0.0);
    }
    
    #[test]
    fn test_sortino_ratio() {
        let returns = vec![0.01, 0.02, -0.01, 0.015, 0.005];
        let sortino = calculate_sortino_ratio(&returns);
        assert!(sortino > 0.0);
    }
}
