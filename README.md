# TraderRank - Trading Analytics System

**Analyze your trading performance with detailed metrics and visualizations**

TraderRank is a trading analytics tool built with Rust that processes CSV/Excel trading data to provide performance metrics, pattern analysis, and terminal-based visualizations.

## Features

### Analytics
- **Performance Metrics**: P&L tracking, win rates, risk-adjusted returns, Sharpe ratios
- **Pattern Recognition**: Identify your most profitable trading hours and patterns
- **Time Analysis**: Discover when you trade best with hourly and session breakdowns
- **Risk Management**: Track maximum drawdowns and risk metrics

### Visualizations
- **Terminal Charts**: P&L charts and win rate trends
- **Calendar Views**: Monthly performance heat maps
- **ASCII Tables**: Color-coded summaries
- **Hourly Distributions**: Intraday performance breakdown

### Performance
- Process 100K+ trades in under 100ms
- Caching for fast historical queries
- Incremental processing - only new data is analyzed
- Memory-efficient architecture

## Quick Start

### Prerequisites
- Rust 1.70+ (install from [rustup.rs](https://rustup.rs))
- CSV or Excel files with trading data

### Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/TraderRank.git
cd TraderRank

# Build the project
cd TraderRank
cargo build --release
```

### Usage

1. **Prepare your data**: Place CSV/Excel files in `Data/Source/` directory

2. **Run the analysis**:
```bash
# Process new trades and show last 10 days
cargo run --release

# Force reprocess all data
cargo run --release -- --reprocess

# Show last 30 days
cargo run --release -- --days 30
```

## Project Structure

```
TraderRank/
├── Data/                 # Data directory
│   ├── Source/          # Input CSV/Excel files
│   └── Processed/       # Cached analysis results
├── TraderRank/          # Main application
│   ├── src/            # Source code
│   │   ├── analytics/  # Trading metrics engine
│   │   ├── models/     # Data structures
│   │   ├── parser/     # CSV/Excel parsing
│   │   ├── persistence/# Data storage
│   │   └── visualization/ # Charts and tables
│   └── Cargo.toml      # Dependencies
├── CHANGES.md          # Version history
└── README.md           # This file
```

## Sample Output

```
TraderRank Analytics Starting...
📂 Processing 1,250 new trades...
✅ Analysis complete!

╔═══════════════════════════════════════════════════╗
║           Daily Trading Summary (Last 10)         ║
╠═══════════════════════════════════════════════════╣
║ Date       │ Trades │ P&L        │ Win Rate      ║
╠═══════════════════════════════════════════════════╣
║ 2024-01-15 │   45   │  $2,456.78 │ 68.9% (31/45) ║
║ 2024-01-14 │   38   │  $1,234.56 │ 65.8% (25/38) ║
║ 2024-01-13 │   52   │ -$567.89   │ 42.3% (22/52) ║
╚═══════════════════════════════════════════════════╝

Daily P&L Chart:
 $3000 ┤      ╭─╮
 $2000 ┤   ╭──╯ ╰╮
 $1000 ┤  ╱      ╰─╮
    $0 ┼─╯         ╰───
-$1000 ┤
       └────────────────
```

## Configuration

### Data Format
Your CSV/Excel files should contain columns for:
- Date/Time
- Symbol
- Side (Buy/Sell)
- Quantity
- Price
- Commission (optional)

The parser automatically detects and adapts to your format.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

Built with these Rust crates:
- `calamine` - Excel parsing
- `tabled` - Beautiful tables
- `textplots` - Terminal charts
- `rust_decimal` - Precise calculations
- `chrono` - Date/time handling

---

Built with Rust