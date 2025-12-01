/// Latency benchmarking for trading strategies
/// Measures microsecond-level performance improvements from template metaprogramming

use std::time::Instant;
use crate::strategy::{Strategy, StrategyContext, KalmanStatArb};
use crate::fast_strategy::FastMM60;
use crate::market::MarketBar;

pub struct StrategyBenchmark {
    iterations: usize,
}

impl StrategyBenchmark {
    pub fn new(iterations: usize) -> Self {
        Self { iterations }
    }

    pub fn benchmark_hot_path(&self, bars: &[MarketBar]) {
        println!("\n{}", "=".repeat(80));
        println!("STRATEGY LATENCY BENCHMARK");
        println!("{}", "=".repeat(80));
        println!("Iterations per strategy: {}", self.iterations);
        println!("Test data: {} bars", bars.len());
        println!();

        // Benchmark original KalmanStatArb
        let mut kalman_strategy = KalmanStatArb::new(60, 100.0, 5.0, 0.2);
        let kalman_latency = self.measure_strategy_latency(&mut kalman_strategy, bars);

        // Benchmark fast strategy
        let mut fast_strategy = FastMM60::new(5.0);
        let fast_latency = self.measure_strategy_latency(&mut fast_strategy, bars);

        // Results
        println!("\n{}", "-".repeat(80));
        println!("RESULTS");
        println!("{}", "-".repeat(80));
        
        println!("\n1. Original Strategy (KalmanStatArb):");
        println!("   Total time:        {:>10.2} ms", kalman_latency.as_secs_f64() * 1000.0);
        println!("   Avg per signal:    {:>10.2} μs", (kalman_latency.as_nanos() as f64) / (self.iterations as f64) / 1000.0);
        println!("   Signals/second:    {:>10.0}", (self.iterations as f64) / kalman_latency.as_secs_f64());

        println!("\n2. Fast Strategy (Template Metaprogramming):");
        println!("   Total time:        {:>10.2} ms", fast_latency.as_secs_f64() * 1000.0);
        println!("   Avg per signal:    {:>10.2} μs", (fast_latency.as_nanos() as f64) / (self.iterations as f64) / 1000.0);
        println!("   Signals/second:    {:>10.0}", (self.iterations as f64) / fast_latency.as_secs_f64());

        let speedup = kalman_latency.as_secs_f64() / fast_latency.as_secs_f64();
        let latency_reduction = ((kalman_latency.as_nanos() as f64 - fast_latency.as_nanos() as f64) 
            / kalman_latency.as_nanos() as f64) * 100.0;

        println!("\n{}", "-".repeat(80));
        println!("PERFORMANCE GAIN");
        println!("{}", "-".repeat(80));
        println!("   Speedup:           {:>10.2}x faster", speedup);
        println!("   Latency reduction: {:>10.2}%", latency_reduction);
        println!();

        // Optimization breakdown
        println!("{}", "-".repeat(80));
        println!("OPTIMIZATION TECHNIQUES APPLIED");
        println!("{}", "-".repeat(80));
        println!("✓ Const Generics:");
        println!("  - Fixed-size arrays at compile time (N=60)");
        println!("  - Zero heap allocations in hot path");
        println!("  - Loop unrolling optimizations");
        println!();
        println!("✓ Inline Functions:");
        println!("  - Critical path functions marked #[inline(always)]");
        println!("  - Eliminates function call overhead");
        println!("  - Better branch prediction");
        println!();
        println!("✓ Memory Layout:");
        println!("  - Cache-aligned structs (#[repr(C, align(32))])");
        println!("  - Stack-allocated ring buffers");
        println!("  - Reduced memory fragmentation");
        println!();
        println!("✓ Algorithm Simplification:");
        println!("  - Diagonal covariance matrix (2x2 vs 3x3)");
        println!("  - Simplified Kalman gain calculation");
        println!("  - Eliminated matrix inversions");
        println!("{}", "=".repeat(80));
    }

    fn measure_strategy_latency<S: Strategy>(&self, strategy: &mut S, bars: &[MarketBar]) -> std::time::Duration {
        strategy.reset();
        
        // Warmup
        for i in 0..10.min(bars.len()) {
            let context = StrategyContext {
                bars,
                current_index: i,
                position: None,
            };
            let _ = strategy.generate_signal(&context);
        }

        // Actual benchmark
        let start = Instant::now();
        for _ in 0..self.iterations {
            for i in 0..bars.len() {
                let context = StrategyContext {
                    bars,
                    current_index: i,
                    position: None,
                };
                let _ = strategy.generate_signal(&context);
            }
            strategy.reset();
        }
        start.elapsed()
    }
}
