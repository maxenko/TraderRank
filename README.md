# TraderRank - Trading Analytics System

**Analyze your trading performance with detailed metrics and visualizations**

TraderRank is a trading analytics tool built with Rust that processes CSV/Excel trading data to provide performance metrics, pattern analysis, and terminal-based visualizations.

## Features

### Analytics
- **Performance Metrics**: P&L tracking (net & gross), win rates, commission analysis
- **Pattern Recognition**: Identify your most profitable trading hours and market sessions
- **Time Analysis**: Discover when you trade best with hourly, daily, and weekly breakdowns
- **Position Management**: Sophisticated trade matching for long/short positions

### Visualizations
- **Terminal Charts**: P&L trends, win rate charts, commission impact analysis
- **Calendar Views**: Dual monthly calendars showing net vs gross P&L comparison
- **Weekly Analysis**: Comprehensive weekly performance tracking with trends
- **ASCII Tables**: Color-coded summaries with detailed and brief formats
- **Hourly Distributions**: Intraday performance breakdown by market sessions

### Data Management
- **Smart Processing**: Incremental processing - only new data is analyzed
- **Caching System**: JSON-based caching for fast historical queries
- **Duplicate Detection**: Automatic filtering of duplicate trades
- **File Tracking**: Intelligent tracking of processed files to avoid reprocessing

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

# The system automatically:
# - Detects new CSV files in Data/Source/
# - Filters duplicate trades
# - Generates comprehensive analytics
# - Caches results for fast retrieval
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
🚀 TraderRank Analytics Engine Starting...
📂 Checking for new trade data...
🔍 Found 2 new file(s) to process
  📄 Processing: trades_2024.csv
     └─ 1250 trades found
✅ Processing 1250 unique trades (filtered 0 duplicates)
🧮 Analyzing trading performance...
💾 Saving analysis results...
📊 Generating reports...

═══════ Overall Trading Summary ═══════
├─ Total Net P&L: $12,456.78 (Gross: $13,456.78, Commissions: -$1,000.00)
├─ Win Rate: 68.5% (856/1250 trades)
├─ Average Win: $45.67
├─ Average Loss: -$23.45
├─ Best Day: 2024-01-15 ($2,456.78)
└─ Worst Day: 2024-01-13 (-$567.89)

📈 Daily P&L Trend (Last 10 Days)
📊 Daily Win Rate Trend
📅 Monthly Calendar Views (Net vs Gross)
📊 Weekly Performance Analysis

🎯 Best Trading Periods Analysis
🥇 Market Open (09:00-10:00): $5,234.56 | Win Rate: 72.3%
🥈 Power Hour (15:00-16:00): $3,456.78 | Win Rate: 68.9%
🥉 Lunch Hour (12:00-13:00): $2,345.67 | Win Rate: 65.4%

✨ Analysis complete!
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