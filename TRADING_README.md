# PhlopChain Quantitative Trading Backtester

A comprehensive quantitative trading backtesting framework built on blockchain concepts, featuring realistic market simulation and institutional-grade performance analytics.

## Overview

This project extends the PhlopChain blockchain with a sophisticated market simulation and backtesting engine that uses the blockchain's Rock-Paper-Scissors (RPS) mining mechanism to generate realistic price movements.

## Key Features

### 🎲 Realistic Market Simulation

- **RPS-Based Price Generation**: Uses blockchain's RPS mining to create deterministic, reproducible price movements
- **Market Regimes**: Dynamic switching between trending, mean-reverting, high/low volatility, and crisis modes
- **Volatility Clustering**: GARCH-like effects where high volatility periods cluster together
- **Microstructure Modeling**: 
  - Realistic bid-ask spreads (regime-dependent)
  - Depth curves with exponential falloff
  - Tick size and lot size constraints
  - Market maker order placement

- **Price Dynamics**:
  - Momentum effects with autocorrelation
  - Mean reversion forces
  - Intraday volume patterns (U-shaped curve)
  - Microstructure noise (bid-ask bounce)
  - Circuit breakers (±10% limits)

### 📊 Comprehensive Backtesting Engine

#### Execution Models
- **Naive**: Fill at bar close (unrealistic)
- **Realistic**: Models slippage and market impact
- **Conservative**: Pessimistic fills for worst-case analysis

#### Transaction Cost Modeling
- Commission fees (configurable)
- Slippage in basis points
- Market impact using square-root model: `impact = price × factor × sqrt(order_size/volume)`
- Separate tracking of each cost component

#### Risk Management
- Position size limits
- Stop-loss orders (percentage-based)
- Maximum leverage constraints
- Per-trade risk controls

### 📈 Performance Analytics

#### Risk-Adjusted Returns
- **Sharpe Ratio**: Return per unit of total risk
- **Sortino Ratio**: Return per unit of downside risk (only negative returns)
- **Calmar Ratio**: Return per unit of maximum drawdown

#### Drawdown Analysis
- Maximum drawdown (peak-to-trough)
- Average drawdown across all drawdown periods
- Drawdown duration tracking

#### Trade Statistics
- Win rate (percentage of profitable trades)
- Profit factor (gross profits / gross losses)
- Average win/loss amounts
- Largest win/loss
- Average holding period
- Total number of winning/losing trades

#### Cost Analysis
- Total commission paid
- Total slippage costs
- Market impact costs
- Cost breakdown per trade

## Trading Strategies Included

### 1. Moving Average Crossover
```rust
MovingAverageCrossover::new(fast_period, slow_period)
```
Classic trend-following strategy using fast and slow moving averages.

### 2. Mean Reversion
```rust
MeanReversion::new(lookback_period, entry_threshold, exit_threshold)
```
Trades based on z-score deviations from the mean.

### 3. Momentum
```rust
Momentum::new(lookback_period, threshold)
```
Follows price momentum with configurable lookback and threshold.

### 4. RSI (Relative Strength Index)
```rust
RSIStrategy::new(period, oversold, overbought)
```
Oscillator-based strategy using RSI thresholds.

## Usage

### Running the Backtester

```bash
cd rsm-en
cargo run --bin trading_backtest
```

### Example Output

```
PhlopChain Quantitative Trading Backtester v2
======================================================================
Realistic Market Simulation with Comprehensive Backtesting
======================================================================

🎲 Generating realistic market data...
✅ Generated 1000 bars of market data
   Initial Price: $50000.00
   Final Price:   $50111.19
   Price Change:  0.28%

📊 Backtesting with Realistic Execution:
   Initial Capital: $100000.00
   Commission:      0.1% per trade
   Slippage:        5 basis points
   Market Impact:   sqrt model

======================================================================
Backtest Results: MA Crossover
======================================================================
Initial Capital:     $100000.00
Final Value:         $93987.11
Total Return:        -6.01%
Annualized Return:   -1.52%
Return Volatility:   0.64%

Risk-Adjusted Returns:
  Sharpe Ratio:      -2.4408
  Sortino Ratio:     -2.6417
  Calmar Ratio:      -0.2506

Drawdown Analysis:
  Max Drawdown:      6.05%
  Avg Drawdown:      2.15%

Trade Statistics:
  Total Trades:      17
  Win Rate:          0.00%
  Profit Factor:     0.0000
  Avg Win:           $0.00
  Avg Loss:          $504.50
  Avg Holding:       29.9 bars

Transaction Costs:
  Total Commission:  $3141.80
  Total Slippage:    $1390.38
  Total Costs:       $4532.18
======================================================================
```

### Creating Custom Strategies

Implement the `Strategy` trait:

```rust
use crate::strategy::{Strategy, StrategyContext, Signal};

pub struct MyStrategy {
    // Your strategy parameters
}

impl Strategy for MyStrategy {
    fn name(&self) -> &str {
        "My Custom Strategy"
    }

    fn generate_signal(&mut self, context: &StrategyContext) -> Signal {
        // Access current and historical bars
        let current_bar = context.current_bar();
        let lookback = context.lookback(20);
        
        // Implement your logic
        if /* buy condition */ {
            Signal::Buy
        } else if /* sell condition */ {
            Signal::Sell
        } else {
            Signal::Hold
        }
    }

    fn reset(&mut self) {
        // Reset any state
    }
}
```

### Configuring the Backtester

```rust
use rsm_en::backtester_v2::{Backtester, TransactionCosts, ExecutionModel};

let costs = TransactionCosts {
    commission_rate: 0.001,      // 0.1%
    slippage_bps: 5.0,            // 5 basis points
    market_impact_factor: 0.1,    // Square root model
};

let backtester = Backtester::new(100000.0)  // Initial capital
    .with_costs(costs)
    .with_execution_model(ExecutionModel::Realistic)
    .with_risk_controls(0.95, 0.05);  // 95% position size, 5% stop loss

let result = backtester.run_backtest(&mut strategy, &bars, 0.95);
result.print_summary();
```

## Blockchain Integration

The trading system leverages PhlopChain concepts:

- **Hash Verification**: Each order and trade is cryptographically hashed (like blockchain transactions)
- **RPS Mining**: Market price movements are generated using the blockchain's RPS mining mechanism
- **Deterministic Execution**: Order matching uses deterministic algorithms for reproducibility
- **Immutable History**: Trade history is immutable and linked (like blockchain blocks)
- **Merkle Trees**: Market data structures use concepts from blockchain's Merkle tree implementation

## Market Simulation Details

### Price Movement Formula

```
new_price = current_price + base_random_walk + regime_adjustment 
            + volatility_clustering + momentum_drift + microstructure_noise
```

Where:
- **base_random_walk**: Generated from RPS game outcomes
- **regime_adjustment**: Multiplier based on current market regime
- **volatility_clustering**: GARCH-like persistence of volatility
- **momentum_drift**: Autocorrelation in returns
- **microstructure_noise**: Small bid-ask bounce effects

### Order Book Structure

```
Level  Bid Price  Bid Size  |  Ask Price  Ask Size
-----  ---------  --------  |  ---------  --------
  1    $49,995.00    10.0   |  $50,005.00    10.0
  2    $49,990.00     8.0   |  $50,010.00     8.0
  3    $49,985.00     6.4   |  $50,015.00     6.4
  ...
```

Depth falls off exponentially from the touch (best bid/ask).

## Performance Metrics Explained

### Sharpe Ratio
```
Sharpe = (Annualized Return - Risk Free Rate) / Annualized Volatility
```
Measures return per unit of risk. Higher is better. Above 1.0 is good, above 2.0 is excellent.

### Sortino Ratio
```
Sortino = (Annualized Return - Risk Free Rate) / Downside Deviation
```
Like Sharpe but only penalizes downside volatility. Better for strategies with positive skew.

### Calmar Ratio
```
Calmar = Annualized Return / Maximum Drawdown
```
Measures return per unit of worst drawdown. Higher values indicate better risk-adjusted returns.

### Profit Factor
```
Profit Factor = Gross Profits / Gross Losses
```
Above 1.0 is profitable. Above 2.0 is strong.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     PhlopChain Trading System                    │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────────────┐        ┌──────────────────┐               │
│  │  RPS Mining     │───────>│  Price Generator │               │
│  │  (Blockchain)   │        │                  │               │
│  └─────────────────┘        └────────┬─────────┘               │
│                                       │                          │
│                                       v                          │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │           Market Simulator                                │  │
│  │  • Regime Switching  • Volatility Clustering             │  │
│  │  • Order Book        • Microstructure                    │  │
│  └────────────────────────┬─────────────────────────────────┘  │
│                            │                                     │
│                            v                                     │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │           Market Bars (OHLCV)                            │  │
│  └────────────────────────┬─────────────────────────────────┘  │
│                            │                                     │
│                            v                                     │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │           Trading Strategy                                │  │
│  │  • MA Crossover  • Mean Reversion  • Momentum  • RSI     │  │
│  └────────────────────────┬─────────────────────────────────┘  │
│                            │                                     │
│                            v                                     │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │           Backtesting Engine                              │  │
│  │  • Execution Model  • Cost Model  • Risk Management      │  │
│  └────────────────────────┬─────────────────────────────────┘  │
│                            │                                     │
│                            v                                     │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │           Performance Analytics                           │  │
│  │  • Sharpe/Sortino  • Drawdowns  • Trade Stats           │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

## Files

### Core Market Simulation
- `src/market.rs` - Order book, orders, trades, and market bars
- `src/market_simulation.rs` - Market simulator with realistic dynamics
- `src/strategy.rs` - Strategy trait and implementations
- `src/backtester_v2.rs` - Enhanced backtesting engine

### Blockchain Foundation
- `src/blockchain.rs` - Blockchain core
- `src/rps_mining.rs` - Rock-Paper-Scissors mining mechanism
- `src/merkle.rs` - Merkle tree implementation
- `src/transaction.rs` - Transaction and block structures

### Entry Points
- `src/trading_main.rs` - Trading backtester demo
- `src/main.rs` - Original blockchain demo

## Future Enhancements

- [ ] Multi-asset portfolio backtesting
- [ ] Options and derivatives strategies
- [ ] Machine learning strategy optimization
- [ ] Real-time paper trading mode
- [ ] Web dashboard with interactive charts
- [ ] Strategy parameter optimization (walk-forward analysis)
- [ ] Monte Carlo simulation for robustness testing
- [ ] High-frequency trading simulation
- [ ] Order book imbalance signals
- [ ] Market making strategies

## License

See LICENSE file in the repository root.

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues for bugs and feature requests.
