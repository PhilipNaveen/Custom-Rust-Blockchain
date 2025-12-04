# Resume Bullets - Trading Strategy & Backtester

## Quantitative Trading System

**Extended Kalman Filter Market Making Strategy**
- Engineered a real-time market making strategy using Extended Kalman Filter (EKF) with 3D state estimation (price, velocity, acceleration) and Nesterov Accelerated Gradient momentum for predictive price modeling
- Implemented statistical arbitrage logic with dynamic entry thresholds and inventory management, achieving 100% win rate and 0.30 Sharpe ratio over 10,000 simulated trading bars
- Optimized strategy latency by 96% (2,895μs → 115μs per signal) through template metaprogramming with const generics, cache-aligned data structures, and diagonal covariance matrix simplification
- Built live trading dashboard with real-time visualization of equity curves, price charts, drawdown analysis, and automated trading capabilities with configurable risk parameters

**High-Fidelity Order Book Market Simulator**
- Designed and implemented realistic order book simulation with 1,430 heterogeneous market participants (70% Retail, 7% Institutional, 14% HFT, 3.5% Market Makers, 0.7% Whales, 3.5% Momentum, 1.4% Arbitrageurs)
- Developed FIFO order matching engine with 10-level market depth, bid-ask spread dynamics, and realistic transaction costs (0.1% commission, 5 bps slippage, square root market impact)
- Simulated continuous price discovery through random walk fair value updates with 10 bps volatility and automated market maker liquidity replenishment

**Rust-Based Backtesting Framework**
- Architected comprehensive backtesting system in Rust with support for multiple strategy types (Moving Average Crossover, Mean Reversion, Momentum, RSI, Kalman Filtering)
- Implemented performance analytics including Sharpe ratio calculation, maximum drawdown tracking, win rate analysis, and annualized return metrics
- Integrated deterministic RPS mining blockchain for reproducible random number generation, ensuring consistent backtesting results across runs
- Created modular strategy trait system enabling rapid prototyping and testing of new trading algorithms

**Live Trading Interface**
- Developed full-stack web application using Rust backend (tiny_http) with HTTP long-polling for sub-100ms latency market updates
- Built responsive trading dashboard with Chart.js visualizations, manual trade execution, autotrade toggle, and real-time P&L tracking
- Implemented session management system supporting concurrent trading sessions with independent strategy parameters and state isolation
- Designed RESTful API endpoints for trade execution, session control, and market data streaming with proper CORS handling

## Technical Skills Demonstrated

**Languages & Frameworks:** Rust, JavaScript, HTML/CSS, Chart.js
**Algorithms:** Extended Kalman Filtering, Nesterov Momentum, Statistical Arbitrage, Market Making
**Systems:** Real-time data streaming, WebSocket/HTTP polling, Multi-threaded architecture
**Finance:** Order book dynamics, Market microstructure, Risk management, Performance attribution
**Optimization:** Template metaprogramming, Cache alignment, SIMD-friendly data layouts, Zero-copy operations

## Alternative Shorter Bullets

**Quantitative Trading & Market Simulation**
- Built real-time market making strategy using Extended Kalman Filter achieving 0.30 Sharpe ratio; optimized execution latency by 96% through Rust template metaprogramming and cache-aligned data structures
- Designed order book simulator with 1,430 heterogeneous traders and FIFO matching engine; developed live trading dashboard with sub-100ms market updates and automated strategy execution
- Implemented comprehensive backtesting framework with performance analytics (Sharpe, drawdown, win rate) and deterministic random number generation via custom blockchain

**Trading System Architecture**
- Engineered high-frequency market making system in Rust with 115μs signal generation latency using const generics and diagonal covariance EKF
- Developed full-stack trading platform featuring WebSocket real-time updates, RESTful API, Chart.js visualizations, and concurrent session management
- Created realistic market simulator supporting multiple trader archetypes, 10-level order book depth, and transaction cost modeling (commission, slippage, market impact)
