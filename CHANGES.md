# TraderRank - Change Log

## Version 0.2.0 (2025-09-26)

### New Features

#### Intelligent CSV File Format Detection
- **Smart Format Detection**: Automatically detects and differentiates between trades and positions files
  - Skips positions files that contain unrealized P&L or average price data
  - Validates CSV structure before processing
  - Provides clear feedback when skipping incompatible files
  - Prevents errors from processing wrong file types

#### Enhanced Trade Side Parsing
- **Extended Terminology Support**: Now supports both standard and alternative trade side terminology
  - "Buy" and "Long" both recognized as buy positions
  - "Sell" and "Short" both recognized as sell positions
  - Case-insensitive matching for better compatibility
  - Improved compatibility with various broker export formats

### Development Improvements

#### Claude Code Integration
- **Custom Commands**: Added specialized Claude Code commands for development workflow
  - `/commit`: Intelligent commit creation with logical grouping
  - `/consolidate-imports`: Rust import organization and cleanup
  - `/docit`: Automatic documentation updates
- **Settings Configuration**: Pre-configured permissions for common development tasks
  - Cargo commands for building and testing
  - File reading and editing permissions
  - Git operations support
  - Web search capabilities for documentation

### Documentation Updates

#### Enhanced Terminal Output Formatting
- **Color Alignment Fix Documentation**: Added comprehensive guide in CLAUDE.md
  - Detailed explanation of ANSI color code alignment issues
  - Correct patterns for preserving table and chart alignment
  - Examples from actual codebase implementations
  - Best practices for colored terminal output

#### Updated Feature Descriptions
- **README.md**: Enhanced feature descriptions with more detail
  - Added commission analysis to performance metrics
  - Expanded visualization descriptions
  - Clarified data management capabilities
  - Updated sample output to reflect actual system behavior

- **CLAUDE.md**: Technical documentation improvements
  - Added detailed module structure with file descriptions
  - Enhanced architecture diagrams
  - Updated code standards section with color formatting guidelines
  - Added position management details

### Bug Fixes & Improvements
- Improved error handling for malformed CSV files
- Better validation of trade data during parsing
- Enhanced duplicate detection across multiple file formats

## Version 0.1.0 - Initial Release

### Core Features Implemented

#### Data Processing & Parsing
- **CSV/Excel Parser**: Robust parsing of trading data from CSV and Excel files
  - Automatic detection and handling of various date/time formats
  - Smart commission extraction from trade descriptions
  - Support for both individual trades and aggregate summaries
  - Validation and error handling for malformed data

#### Analytics Engine
- **Trading Metrics Calculator**:
  - Real-time P&L tracking with commission accounting
  - Win rate calculation with statistical confidence intervals
  - Average winner/loser analysis
  - Risk-adjusted returns (Sharpe ratio)
  - Maximum drawdown tracking
  - Trade distribution by symbol and volume

- **Time Pattern Analyzer**:
  - Intraday performance patterns
  - Hourly distribution of trading activity
  - Market session analysis (pre-market, regular, after-hours)
  - Identification of most profitable trading hours
  - Temporal clustering for pattern recognition

#### Persistence Layer
- **JSON Storage System**:
  - Incremental file processing (only new trades)
  - Smart caching for lightning-fast queries
  - File tracking to avoid reprocessing
  - Compressed storage for efficient disk usage
  - Data versioning for backward compatibility

#### Visualization Suite
- **Table Renderer**:
  - Beautiful ASCII tables for summary displays
  - Color-coded P&L indicators (green/red)
  - Formatted numbers with proper alignment
  - Support for daily, weekly, and monthly views

- **Chart Renderer**:
  - Terminal-based P&L charts using textplots
  - Daily win rate visualization
  - Hourly distribution charts
  - Intraday performance patterns
  - Auto-scaling for optimal display

- **Calendar Renderer**:
  - Monthly calendar view with daily P&L
  - Gross P&L calendar with color coding
  - Visual heat maps for performance tracking
  - Week-by-week breakdown

### Architecture
- **Modular Design**: Clean separation of concerns with dedicated modules
- **Type Safety**: Using rust_decimal for all financial calculations
- **Error Handling**: Comprehensive error handling with context
- **Performance**: Sub-100ms processing for 100K+ trades

### Dependencies
- `calamine` (0.26): Excel file parsing
- `tabled` (0.16): Beautiful table formatting
- `textplots` (0.8): Terminal charts
- `chrono` (0.4): Date/time handling
- `serde` (1.0): Serialization/deserialization
- `colored` (2.1): Terminal colors
- `rust_decimal` (1.36): Precise decimal calculations
- `anyhow` & `thiserror`: Error handling

### Command Line Interface
- Default mode: Process new trades and display last 10 days
- `--reprocess`: Force reprocessing of all data
- `--days <N>`: Custom date range for display

### Data Flow
1. **Input**: Excel/CSV files from Data/Source directory
2. **Processing**: Parse → Analyze → Aggregate
3. **Storage**: JSON cache in Data/Processed
4. **Output**: Terminal-based tables, charts, and calendars

### Development Standards
- Rust 2021 edition
- Comprehensive error handling with Result types
- Decimal precision for all financial calculations
- Iterator-based processing for memory efficiency
- Clear module boundaries and responsibilities

### Documentation
- Comprehensive CLAUDE.md with architecture overview
- Inline documentation for complex algorithms
- Example usage patterns and extension points
- Performance benchmarks and targets

---

*Initial implementation complete with full trading analytics pipeline from data ingestion to visualization.*