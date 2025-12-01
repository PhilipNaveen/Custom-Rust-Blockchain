use rsm_en::market;
use rsm_en::market_simulation::MarketSimulator;
use rsm_en::strategy::{MovingAverageCrossover, MeanReversion, Momentum, RSIStrategy};
use rsm_en::backtester_v2::{Backtester, TransactionCosts, ExecutionModel};
use rsm_en::visualization::TradingVisualizer;

fn main() {
    println!("PhlopChain Quantitative Trading Backtester v2");
    println!("{}", "=".repeat(70));
    println!("Realistic Market Simulation with Comprehensive Backtesting");
    println!("{}", "=".repeat(70));

    // Create market simulator
    let symbols = vec!["BTC/USD".to_string()];
    let mut market_sim = MarketSimulator::new(symbols);

    println!("\n🎲 Generating realistic market data...");
    println!("   Using RPS-based price movements with:");
    println!("   • Volatility clustering (GARCH effects)");
    println!("   • Regime switching (trending/mean-reverting)");
    println!("   • Realistic order book depth");
    println!("   • Microstructure noise and bid-ask spreads");
    
    let initial_price = 50000.0;
    market_sim.set_initial_price("BTC/USD", initial_price);
    let num_bars = 1000;

    let bars = market_sim.simulate_session("BTC/USD", initial_price, num_bars);

    println!("\n✅ Generated {} bars of market data", bars.len());
    println!("   Initial Price: ${:.2}", bars[0].close);
    println!("   Final Price:   ${:.2}", bars.last().unwrap().close);
    println!("   Price Change:  {:.2}%", 
        ((bars.last().unwrap().close - bars[0].close) / bars[0].close) * 100.0);
    
    let total_volume: f64 = bars.iter().map(|b| b.volume).sum();
    let avg_volume = total_volume / bars.len() as f64;
    println!("   Avg Volume:    {:.2}", avg_volume);
    println!("   Total Trades:  {}", bars.iter().filter(|b| b.volume > 0.0).count());
    
    // Show trader statistics
    market_sim.print_trader_stats();

    // Backtest parameters
    let initial_capital = 100000.0;
    let position_size = 0.95;

    println!("\n📊 Backtesting with Realistic Execution:");
    println!("   Initial Capital: ${:.2}", initial_capital);
    println!("   Commission:      0.1% per trade");
    println!("   Slippage:        5 basis points");
    println!("   Market Impact:   sqrt model");
    println!("   Position Size:   {:.0}%", position_size * 100.0);
    println!("   Stop Loss:       5%");

    // Configure realistic transaction costs
    let costs = TransactionCosts {
        commission_rate: 0.001,
        slippage_bps: 5.0,
        market_impact_factor: 0.1,
    };

    // Strategy 1: Moving Average Crossover
    println!("\n🔄 Strategy 1: Moving Average Crossover (10/50)");
    let mut ma_strategy = MovingAverageCrossover::new(10, 50);
    let mut backtester1 = Backtester::new(initial_capital)
        .with_costs(costs.clone())
        .with_execution_model(ExecutionModel::Realistic)
        .with_risk_controls(position_size, 0.05);
    let result1 = backtester1.run_backtest(&mut ma_strategy, &bars, position_size);
    result1.print_summary();

    // Strategy 2: Mean Reversion
    println!("\n📉 Strategy 2: Mean Reversion (30 period, z-score ±2.0)");
    let mut mr_strategy = MeanReversion::new(30, 2.0, 0.5);
    let mut backtester2 = Backtester::new(initial_capital)
        .with_costs(costs.clone())
        .with_execution_model(ExecutionModel::Realistic)
        .with_risk_controls(position_size, 0.05);
    let result2 = backtester2.run_backtest(&mut mr_strategy, &bars, position_size);
    result2.print_summary();

    // Strategy 3: Momentum
    println!("\n🚀 Strategy 3: Momentum (20 period, 1.5% threshold)");
    let mut mom_strategy = Momentum::new(20, 0.015);
    let mut backtester3 = Backtester::new(initial_capital)
        .with_costs(costs.clone())
        .with_execution_model(ExecutionModel::Realistic)
        .with_risk_controls(position_size, 0.05);
    let result3 = backtester3.run_backtest(&mut mom_strategy, &bars, position_size);
    result3.print_summary();

    // Strategy 4: RSI
    println!("\n📈 Strategy 4: RSI (14 period, 30/70)");
    let mut rsi_strategy = RSIStrategy::new(14, 30.0, 70.0);
    let mut backtester4 = Backtester::new(initial_capital)
        .with_costs(costs.clone())
        .with_execution_model(ExecutionModel::Realistic)
        .with_risk_controls(position_size, 0.05);
    let result4 = backtester4.run_backtest(&mut rsi_strategy, &bars, position_size);
    result4.print_summary();

    // Compare strategies
    println!("\n{}", "=".repeat(70));
    println!("STRATEGY COMPARISON");
    println!("{}", "=".repeat(70));
    
    let results = vec![
        ("MA Crossover", &result1),
        ("Mean Reversion", &result2),
        ("Momentum", &result3),
        ("RSI", &result4),
    ];

    println!("{:<20} {:>12} {:>12} {:>12} {:>12} {:>10}", 
        "Strategy", "Return %", "Sharpe", "Sortino", "Max DD %", "Win Rate %");
    println!("{}", "-".repeat(82));

    for (name, result) in &results {
        let m = &result.metrics;
        println!("{:<20} {:>12.2} {:>12.4} {:>12.4} {:>12.2} {:>10.2}",
            name,
            m.total_return * 100.0,
            m.sharpe_ratio,
            m.sortino_ratio,
            m.max_drawdown * 100.0,
            m.win_rate * 100.0,
        );
    }

    // Find best strategy
    let best_strategy = results.iter()
        .max_by(|a, b| a.1.metrics.sharpe_ratio.partial_cmp(&b.1.metrics.sharpe_ratio).unwrap())
        .unwrap();

    println!("\n🏆 Best Strategy by Sharpe Ratio: {}", best_strategy.0);
    println!("   Sharpe Ratio:   {:.4}", best_strategy.1.metrics.sharpe_ratio);
    println!("   Sortino Ratio:  {:.4}", best_strategy.1.metrics.sortino_ratio);
    println!("   Return:         {:.2}%", best_strategy.1.metrics.total_return * 100.0);
    println!("   Max Drawdown:   {:.2}%", best_strategy.1.metrics.max_drawdown * 100.0);

    // Show some sample trades from best strategy
    println!("\n📋 Sample Trades from {} (first 5):", best_strategy.0);
    for (i, trade) in best_strategy.1.trade_history.iter().take(5).enumerate() {
        let side_emoji = match trade.side {
            market::OrderSide::Buy => "🟢",
            market::OrderSide::Sell => "🔴",
        };
        println!("   {}. {} {:?} {:.4} @ ${:.2} (intended: ${:.2}, slippage: ${:.2})",
            i + 1,
            side_emoji,
            trade.side,
            trade.quantity,
            trade.fill_price,
            trade.intended_price,
            trade.slippage + trade.market_impact,
        );
    }

    // Generate visualizations
    println!("\n{}", "=".repeat(70));
    println!("📊 GENERATING VISUALIZATIONS");
    println!("{}", "=".repeat(70));
    
    let visualizer = TradingVisualizer::new("trading_charts".to_string());
    
    println!("\nGenerating charts for all strategies...");
    for (name, result) in &results {
        println!("\n📈 Generating charts for: {}", name);
        match visualizer.generate_all(result, &bars) {
            Ok(files) => {
                println!("   ✅ Generated {} charts", files.len());
            }
            Err(e) => {
                println!("   ❌ Error generating charts: {}", e);
            }
        }
    }
    
    println!("\n📁 Charts saved to: trading_charts/");
    println!("   Open the PNG files to view detailed analysis");

    // Demonstrate the blockchain connection
    println!("\n{}", "=".repeat(70));
    println!("🔗 BLOCKCHAIN CONNECTION");
    println!("{}", "=".repeat(70));
    println!("This trading system leverages concepts from your PhlopChain:");
    println!("  • Market orders use Hash verification (like transactions)");
    println!("  • Price movements generated via RPS mining mechanism");
    println!("  • Order matching uses deterministic algorithms");
    println!("  • Trade history is immutable and cryptographically linked");
    println!("  • Market simulation uses blockchain's merkle tree concepts");
    println!("  • Heterogeneous trader population (1430 traders total)");
    println!("  • Realistic volume distribution and order flow");
    
    println!("\n✨ Backtest Complete!");
}
