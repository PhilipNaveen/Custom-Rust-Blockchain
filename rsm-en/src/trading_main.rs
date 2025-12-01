use rsm_en::orderbook_market::OrderBookMarket;
use rsm_en::fast_strategy::FastMM60;
use rsm_en::backtester_v2::{Backtester, TransactionCosts, ExecutionModel};
use rsm_en::visualization::TradingVisualizer;

fn main() {
    println!("PhlopChain Quantitative Trading Backtester - Ultra-Low-Latency Market Making");
    println!("{}", "=".repeat(80));
    println!("Template Metaprogramming: Compile-Time Optimized Trading Strategy");
    println!("{}", "=".repeat(80));

    // Create order book market with realistic depth
    let symbol = "BTC/USD".to_string();
    let initial_price = 50000.0;
    let mut market = OrderBookMarket::new(symbol, initial_price);

    println!("\nGenerating realistic order book market data...");
    println!("   Realistic order book with bid-ask spreads (5-15 bps)");
    println!("   Multiple levels of market depth");
    println!("   Market makers providing liquidity");
    println!("   Heterogeneous trader population (1430 traders)");
    println!("   Real price formation from order matching");
    
    let num_bars = 10000; // Much longer simulation
    let bars = market.simulate_session(num_bars);

    println!("\nGenerated {} bars of market data", bars.len());
    println!("   Initial Price: ${:.2}", bars[0].close);
    println!("   Final Price:   ${:.2}", bars.last().unwrap().close);
    println!("   Price Change:  {:.2}%", 
        ((bars.last().unwrap().close - bars[0].close) / bars[0].close) * 100.0);
    
    let total_volume: f64 = bars.iter().map(|b| b.volume).sum();
    let avg_volume = total_volume / bars.len() as f64;
    println!("   Avg Volume:    {:.2}", avg_volume);
    println!("   Total Trades:  {}", bars.iter().filter(|b| b.volume > 0.0).count());
    println!("   Final Spread:  {:.2} bps", market.get_spread_bps());
    
    // Show trader statistics
    market.print_trader_stats();

    // Backtest parameters
    let initial_capital = 100000.0;
    let position_size = 0.95;

    println!("\n{}", "=".repeat(80));
    println!("BACKTESTING KALMAN FILTER MARKET MAKING");
    println!("{}", "=".repeat(80));
    println!("   Initial Capital: ${:.2}", initial_capital);
    println!("   Commission:      0.1% per trade");
    println!("   Slippage:        5 basis points");
    println!("   Market Impact:   Square root model");
    println!("   Position Size:   {:.0}%", position_size * 100.0);
    println!("   Stop Loss:       5%");

    // Configure realistic transaction costs
    let costs = TransactionCosts {
        commission_rate: 0.001,
        slippage_bps: 5.0,
        market_impact_factor: 0.1,
    };

    // Extended Kalman Filter Market Making Strategy
    println!("\nStrategy Parameters:");
    println!("   Algorithm:          EKF Market Maker");
    println!("   State Model:        Price + Velocity (2D observations)");
    println!("   Lookback Period:    60 bars (volatility estimation)");
    println!("   Spread Width:       50 basis points");
    println!("   Max Inventory:      2.0 units");
    println!("   Inventory Skew:     0.05 (quote adjustment per unit)");
    println!("");
    println!("   EKF Configuration:");
    println!("     State Vector:     [price, velocity, acceleration]");
    println!("     Observations:     [price, velocity] (no noisy acceleration)");
    println!("     Process Noise Q:  [0.01, 0.1, 0.5] (adaptive dynamics)");
    println!("     Measurement R:    0.5 (observation noise)");
    println!("     Nesterov Beta:    0.9 (momentum coefficient)");
    println!("     Nesterov Gamma:   0.999 (lookahead factor)");
    println!("");
    println!("   Market Making Logic:");
    println!("     - Estimate true price + velocity using EKF");
    println!("     - Post bid below estimate, ask above estimate");
    println!("     - Adjust quotes based on velocity (momentum)");
    println!("     - Skew quotes based on inventory (lean against)");
    println!("     - Profit from bid-ask spread");
    println!("     - Maintain inventory neutrality");
    println!("   Optimization: Compile-time template metaprogramming");
    println!("     - Const generics for zero-cost abstractions");
    println!("     - Stack-allocated arrays (no heap allocation)");
    println!("     - Inline hot paths for branch prediction");
    println!("     - Cache-aligned structures");
    println!("     - Diagonal covariance matrix (simplified)");
    
    // Fast market maker with compile-time optimization (lookback=60, max_inventory=5.0)
    let mut kalman_strategy = FastMM60::new(5.0);
    let mut backtester = Backtester::new(initial_capital)
        .with_costs(costs.clone())
        .with_execution_model(ExecutionModel::Realistic)
        .with_risk_controls(position_size, 0.05);
    
    println!("\nRunning backtest...");
    let result = backtester.run_backtest(&mut kalman_strategy, &bars, position_size);
    
    println!("\n{}", "=".repeat(80));
    println!("BACKTEST RESULTS");
    println!("{}", "=".repeat(80));
    result.print_summary();

    // Show sample trades
    println!("\nSample Trades (first 10):");
    println!("{:<5} {:<10} {:<10} {:<12} {:<12} {:<12}", 
        "No.", "Side", "Quantity", "Fill Price", "Intended", "Slippage");
    println!("{}", "-".repeat(80));
    
    for (i, trade) in result.trade_history.iter().take(10).enumerate() {
        println!("{:<5} {:<10?} {:<10.4} ${:<11.2} ${:<11.2} ${:<11.4}",
            i + 1,
            trade.side,
            trade.quantity,
            trade.fill_price,
            trade.intended_price,
            trade.slippage + trade.market_impact,
        );
    }

    // Generate visualizations
    println!("\n{}", "=".repeat(80));
    println!("GENERATING VISUALIZATIONS");
    println!("{}", "=".repeat(80));
    
    let visualizer = TradingVisualizer::new("trading_charts".to_string());
    
    println!("\nGenerating charts...");
    match visualizer.generate_all(&result, &bars) {
        Ok(files) => {
            println!("   Generated {} charts successfully", files.len());
            println!("   Output directory: trading_charts/");
            for file in files {
                println!("      - {}", file);
            }
        }
        Err(e) => {
            println!("   Error generating charts: {}", e);
        }
    }

    // Demonstrate the blockchain connection
    println!("\n{}", "=".repeat(80));
    println!("BLOCKCHAIN INTEGRATION");
    println!("{}", "=".repeat(80));
    println!("This trading system leverages PhlopChain concepts:");
    println!("  - Market orders use cryptographic hash verification");
    println!("  - Price movements generated via RPS mining mechanism");
    println!("  - Order matching uses deterministic algorithms");
    println!("  - Trade history is immutable and cryptographically linked");
    println!("  - Market simulation incorporates merkle tree concepts");
    println!("  - Heterogeneous trader population creates realistic dynamics");
    
    println!("\nBacktest Complete!");
}
