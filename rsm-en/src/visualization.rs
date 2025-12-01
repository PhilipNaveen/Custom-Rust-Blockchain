use crate::backtester_v2::{BacktestResult, PortfolioSnapshot, TradeExecution};
use crate::market::{MarketBar, OrderSide};
use plotters::prelude::*;
use std::error::Error;

pub struct TradingVisualizer {
    output_dir: String,
}

impl TradingVisualizer {
    pub fn new(output_dir: String) -> Self {
        std::fs::create_dir_all(&output_dir).ok();
        Self { output_dir }
    }

    /// Generate all visualizations for a backtest result
    pub fn generate_all(
        &self,
        result: &BacktestResult,
        bars: &[MarketBar],
    ) -> Result<Vec<String>, Box<dyn Error>> {
        let mut files = Vec::new();

        files.push(self.plot_equity_curve(result)?);
        files.push(self.plot_price_and_trades(bars, &result.trade_history)?);
        files.push(self.plot_drawdown(result)?);
        files.push(self.plot_returns_distribution(result)?);
        files.push(self.plot_trade_analysis(&result.trade_history)?);

        Ok(files)
    }

    /// Plot equity curve over time
    pub fn plot_equity_curve(&self, result: &BacktestResult) -> Result<String, Box<dyn Error>> {
        let filename = format!("{}/equity_curve_{}.png", self.output_dir, result.strategy_name.replace(" ", "_"));
        {
            let root = BitMapBackend::new(&filename, (1200, 600)).into_drawing_area();
            root.fill(&WHITE)?;

        let equity_data: Vec<(f64, f64)> = result.portfolio_history.iter()
            .enumerate()
            .map(|(i, snapshot)| (i as f64, snapshot.total_value))
            .collect();

        if equity_data.is_empty() {
            drop(root);
            return Ok(filename);
        }

        let min_value = equity_data.iter().map(|(_, v)| v).fold(f64::INFINITY, |a, &b| a.min(b));
        let max_value = equity_data.iter().map(|(_, v)| v).fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let margin = (max_value - min_value) * 0.1;

        let mut chart = ChartBuilder::on(&root)
            .caption(
                format!("Equity Curve: {}", result.strategy_name),
                ("sans-serif", 30).into_font(),
            )
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(
                0.0..(equity_data.len() as f64),
                (min_value - margin)..(max_value + margin),
            )?;

        chart
            .configure_mesh()
            .x_desc("Time (bars)")
            .y_desc("Portfolio Value ($)")
            .draw()?;

        // Plot equity line
        chart.draw_series(LineSeries::new(
            equity_data.iter().map(|(x, y)| (*x, *y)),
            &BLUE.mix(0.8),
        ))?
        .label("Equity")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &BLUE));

        // Plot initial capital line
        chart.draw_series(LineSeries::new(
            vec![(0.0, result.initial_capital), (equity_data.len() as f64, result.initial_capital)],
            RED.mix(0.5).stroke_width(2),
        ))?
        .label("Initial Capital")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &RED));

            chart.configure_series_labels()
                .background_style(&WHITE.mix(0.8))
                .border_style(&BLACK)
                .draw()?;

            root.present()?;
        }
        println!("Generated: {}", filename);
        Ok(filename)
    }

    /// Plot price chart with trade markers
    pub fn plot_price_and_trades(
        &self,
        bars: &[MarketBar],
        trades: &[TradeExecution],
    ) -> Result<String, Box<dyn Error>> {
        let filename = format!("{}/price_and_trades.png", self.output_dir);
        {
            let root = BitMapBackend::new(&filename, (1400, 700)).into_drawing_area();
            root.fill(&WHITE)?;

        let (upper, lower) = root.split_vertically(500);

        // Price chart
        let prices: Vec<(f64, f64)> = bars.iter()
            .enumerate()
            .map(|(i, bar)| (i as f64, bar.close))
            .collect();

        if prices.is_empty() {
            drop(upper);
            drop(lower);
            drop(root);
            return Ok(filename);
        }

        let min_price = prices.iter().map(|(_, p)| p).fold(f64::INFINITY, |a, &b| a.min(b));
        let max_price = prices.iter().map(|(_, p)| p).fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let price_margin = (max_price - min_price) * 0.1;

        let mut price_chart = ChartBuilder::on(&upper)
            .caption("Price Chart with Trades", ("sans-serif", 30).into_font())
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(
                0.0..(bars.len() as f64),
                (min_price - price_margin)..(max_price + price_margin),
            )?;

        price_chart
            .configure_mesh()
            .x_desc("Time (bars)")
            .y_desc("Price ($)")
            .draw()?;

        // Plot price line
        price_chart.draw_series(LineSeries::new(
            prices.iter().map(|(x, y)| (*x, *y)),
            &BLACK.mix(0.8),
        ))?;

        // Create a mapping of timestamp to bar index
        let timestamp_to_index: std::collections::HashMap<u64, usize> = bars.iter()
            .enumerate()
            .map(|(i, bar)| (bar.timestamp, i))
            .collect();

        // Plot buy trades as green circles
        for trade in trades.iter().filter(|t| t.side == OrderSide::Buy) {
            if let Some(&idx) = timestamp_to_index.get(&trade.timestamp) {
                price_chart.draw_series(PointSeries::of_element(
                    vec![(idx as f64, trade.fill_price)],
                    5,
                    ShapeStyle::from(&GREEN).filled(),
                    &|coord, size, style| {
                        EmptyElement::at(coord)
                            + Circle::new((0, 0), size, style)
                    },
                ))?;
            }
        }

        // Plot sell trades as red circles
        for trade in trades.iter().filter(|t| t.side == OrderSide::Sell) {
            if let Some(&idx) = timestamp_to_index.get(&trade.timestamp) {
                price_chart.draw_series(PointSeries::of_element(
                    vec![(idx as f64, trade.fill_price)],
                    5,
                    ShapeStyle::from(&RED).filled(),
                    &|coord, size, style| {
                        EmptyElement::at(coord)
                            + Circle::new((0, 0), size, style)
                    },
                ))?;
            }
        }

        // Volume chart
        let volumes: Vec<(f64, f64)> = bars.iter()
            .enumerate()
            .map(|(i, bar)| (i as f64, bar.volume))
            .collect();

        let max_volume = volumes.iter().map(|(_, v)| v).fold(f64::NEG_INFINITY, |a, &b| a.max(b));

        let mut volume_chart = ChartBuilder::on(&lower)
            .caption("Volume", ("sans-serif", 20).into_font())
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(
                0.0..(bars.len() as f64),
                0.0..(max_volume * 1.1),
            )?;

        volume_chart
            .configure_mesh()
            .x_desc("Time (bars)")
            .y_desc("Volume")
            .draw()?;

        // Draw volume bars
        volume_chart.draw_series(
            volumes.iter().map(|(x, vol)| {
                let mut bar = Rectangle::new([(*x - 0.4, 0.0), (*x + 0.4, *vol)], BLUE.mix(0.6).filled());
                bar.set_margin(0, 0, 0, 0);
                bar
            }),
        )?;

            root.present()?;
        }
        println!("Generated: {}", filename);
        Ok(filename)
    }

    /// Plot drawdown over time
    pub fn plot_drawdown(&self, result: &BacktestResult) -> Result<String, Box<dyn Error>> {
        let filename = format!("{}/drawdown_{}.png", self.output_dir, result.strategy_name.replace(" ", "_"));
        {
            let root = BitMapBackend::new(&filename, (1200, 600)).into_drawing_area();
            root.fill(&WHITE)?;

        // Calculate drawdown series
        let mut max_value = result.initial_capital;
        let mut drawdowns = Vec::new();

        for (i, snapshot) in result.portfolio_history.iter().enumerate() {
            if snapshot.total_value > max_value {
                max_value = snapshot.total_value;
            }
            let drawdown = ((snapshot.total_value - max_value) / max_value) * 100.0;
            drawdowns.push((i as f64, drawdown));
        }

        if drawdowns.is_empty() {
            drop(root);
            return Ok(filename);
        }

        let max_dd = drawdowns.iter().map(|(_, dd)| dd).fold(0.0_f64, |a, &b| a.min(b));

        let mut chart = ChartBuilder::on(&root)
            .caption(
                format!("Drawdown: {}", result.strategy_name),
                ("sans-serif", 30).into_font(),
            )
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(
                0.0..(drawdowns.len() as f64),
                (max_dd * 1.2)..0.0,
            )?;

        chart
            .configure_mesh()
            .x_desc("Time (bars)")
            .y_desc("Drawdown (%)")
            .draw()?;

        // Fill area under drawdown
        chart.draw_series(
            AreaSeries::new(
                drawdowns.iter().map(|(x, y)| (*x, *y)),
                0.0,
                &RED.mix(0.3),
            )
        )?;

        // Draw drawdown line
            chart.draw_series(LineSeries::new(
                drawdowns.iter().map(|(x, y)| (*x, *y)),
                RED.mix(0.8).stroke_width(2),
            ))?;

            root.present()?;
        }
        println!("Generated: {}", filename);
        Ok(filename)
    }

    /// Plot returns distribution histogram
    pub fn plot_returns_distribution(&self, result: &BacktestResult) -> Result<String, Box<dyn Error>> {
        let filename = format!("{}/returns_dist_{}.png", self.output_dir, result.strategy_name.replace(" ", "_"));
        {
            let root = BitMapBackend::new(&filename, (1000, 600)).into_drawing_area();
            root.fill(&WHITE)?;

        // Calculate returns
        let returns: Vec<f64> = result.portfolio_history.windows(2)
            .map(|w| {
                if w[0].total_value == 0.0 {
                    0.0
                } else {
                    ((w[1].total_value - w[0].total_value) / w[0].total_value) * 100.0
                }
            })
            .collect();

        if returns.is_empty() {
            drop(root);
            return Ok(filename);
        }

        // Create histogram bins
        let min_return = returns.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_return = returns.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let num_bins = 50;
        let bin_width = (max_return - min_return) / num_bins as f64;

        let mut bins = vec![0; num_bins];
        for &ret in &returns {
            let bin = ((ret - min_return) / bin_width).floor() as usize;
            let bin = bin.min(num_bins - 1);
            bins[bin] += 1;
        }

        let max_count = *bins.iter().max().unwrap_or(&1);

        let mut chart = ChartBuilder::on(&root)
            .caption(
                format!("Returns Distribution: {}", result.strategy_name),
                ("sans-serif", 30).into_font(),
            )
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(
                min_return..max_return,
                0.0..(max_count as f64 * 1.1),
            )?;

        chart
            .configure_mesh()
            .x_desc("Return (%)")
            .y_desc("Frequency")
            .draw()?;

        // Draw histogram bars
        chart.draw_series(
            bins.iter().enumerate().map(|(i, &count)| {
                let x = min_return + (i as f64 * bin_width);
                Rectangle::new(
                    [(x, 0.0), (x + bin_width, count as f64)],
                    BLUE.mix(0.6).filled(),
                )
            }),
        )?;

        // Draw zero line
            chart.draw_series(LineSeries::new(
                vec![(0.0, 0.0), (0.0, max_count as f64)],
                BLACK.stroke_width(2),
            ))?;

            root.present()?;
        }
        println!("Generated: {}", filename);
        Ok(filename)
    }

    /// Plot trade analysis
    pub fn plot_trade_analysis(&self, trades: &[TradeExecution]) -> Result<String, Box<dyn Error>> {
        let filename = format!("{}/trade_analysis.png", self.output_dir);
        {
            let root = BitMapBackend::new(&filename, (1400, 600)).into_drawing_area();
            root.fill(&WHITE)?;

        let (left, right) = root.split_horizontally(700);

        // Calculate PnL per trade
        let mut trade_pnls = Vec::new();
        
        if trades.len() < 2 {
            drop(left);
            drop(right);
            drop(root);
            return Ok(filename);
        }
        
        let mut i = 0;

        while i < trades.len() - 1 {
            if trades[i].side == OrderSide::Buy {
                if let Some(sell_idx) = trades[i+1..].iter()
                    .position(|t| t.side == OrderSide::Sell && t.symbol == trades[i].symbol) {
                    
                    let buy_trade = &trades[i];
                    let sell_trade = &trades[i + 1 + sell_idx];
                    
                    let pnl = (sell_trade.fill_price - buy_trade.fill_price) * buy_trade.quantity
                        - buy_trade.commission - sell_trade.commission;
                    
                    trade_pnls.push(pnl);
                    i = i + 1 + sell_idx + 1;
                } else {
                    break;
                }
            } else {
                i += 1;
            }
        }

        if trade_pnls.is_empty() {
            drop(left);
            drop(right);
            drop(root);
            return Ok(filename);
        }

        // Cumulative PnL chart
        let mut cumulative_pnl = 0.0;
        let cumulative: Vec<(f64, f64)> = trade_pnls.iter()
            .enumerate()
            .map(|(i, &pnl)| {
                cumulative_pnl += pnl;
                (i as f64, cumulative_pnl)
            })
            .collect();

        let min_cum = cumulative.iter().map(|(_, v)| v).fold(0.0_f64, |a, &b| a.min(b));
        let max_cum = cumulative.iter().map(|(_, v)| v).fold(0.0_f64, |a, &b| a.max(b));
        let margin = (max_cum - min_cum).abs() * 0.1 + 100.0;

        let mut cum_chart = ChartBuilder::on(&left)
            .caption("Cumulative Trade PnL", ("sans-serif", 25).into_font())
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(
                0.0..(cumulative.len() as f64),
                (min_cum - margin)..(max_cum + margin),
            )?;

        cum_chart
            .configure_mesh()
            .x_desc("Trade Number")
            .y_desc("Cumulative PnL ($)")
            .draw()?;

        cum_chart.draw_series(LineSeries::new(
            cumulative.iter().map(|(x, y)| (*x, *y)),
            BLUE.mix(0.8).stroke_width(2),
        ))?;

        // Zero line
        cum_chart.draw_series(LineSeries::new(
            vec![(0.0, 0.0), (cumulative.len() as f64, 0.0)],
            &BLACK.mix(0.5),
        ))?;

        // Individual trade PnL bars
        let max_pnl = trade_pnls.iter().fold(0.0_f64, |a, &b| a.max(b.abs()));

        let mut pnl_chart = ChartBuilder::on(&right)
            .caption("Individual Trade PnL", ("sans-serif", 25).into_font())
            .margin(10)
            .x_label_area_size(40)
            .y_label_area_size(60)
            .build_cartesian_2d(
                0.0..(trade_pnls.len() as f64),
                (-max_pnl * 1.1)..(max_pnl * 1.1),
            )?;

        pnl_chart
            .configure_mesh()
            .x_desc("Trade Number")
            .y_desc("PnL ($)")
            .draw()?;

        // Draw PnL bars (green for profit, red for loss)
        pnl_chart.draw_series(
            trade_pnls.iter().enumerate().map(|(i, &pnl)| {
                let color = if pnl >= 0.0 { GREEN.mix(0.7) } else { RED.mix(0.7) };
                Rectangle::new(
                    [(i as f64 - 0.4, 0.0), (i as f64 + 0.4, pnl)],
                    color.filled(),
                )
            }),
        )?;

        // Zero line
        pnl_chart.draw_series(LineSeries::new(
            vec![(0.0, 0.0), (trade_pnls.len() as f64, 0.0)],
                &BLACK,
            ))?;

            root.present()?;
        }
        println!("Generated: {}", filename);
        Ok(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visualizer_creation() {
        let viz = TradingVisualizer::new("test_output".to_string());
        assert_eq!(viz.output_dir, "test_output");
    }
}
