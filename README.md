# Solana CLOB (Central Limit Order Book) DEX

A high-performance decentralized limit order book implementation built on Solana using the Anchor framework. This CLOB features centralized liquidity, efficient order matching with zero-copy heap-based orderbooks, comprehensive time-in-force order types (GTC, IOC, FOK), and advanced event processing for real-time market data.

## 🏗️ High-Level Architecture

### Core Components

The CLOB consists of four main architectural layers:

#### 1. Market State

The central `Market` account contains:

- **Base Mint**: Address of the base token (e.g., SOL)
- **Quote Mint**: Address of the quote token (e.g., USDC)
- **Lot Sizes**: Minimum tradeable units for price and quantity
- **Next Order ID**: Global counter for unique order identification
- **Event Queue**: Reference to the event queue for deferred balance updates

#### 2. Token Vaults

Centralized liquidity storage:

- **Base Vault**: PDA-controlled token account holding all base tokens
- **Quote Vault**: PDA-controlled token account holding all quote tokens
- **Centralized Model**: All user funds pooled for efficient matching

#### 3. Order Books (Zero-Copy Heap Implementation)

**Major Improvement**: Upgraded from Vec-based to zero-copy heap-based orderbooks for better performance:

- **Bids Book**: Binary heap for buy orders (Side::Bid) with max-heap ordering
- **Asks Book**: Binary heap for sell orders (Side::Ask) with min-heap ordering
- **Zero-Copy**: Uses `#[zero_copy]` accounts for direct memory access without serialization overhead
- **Price-Time Priority**: Orders sorted by best price first, then earliest timestamp
- **High Performance**: Efficient O(log n) insertions and O(1) peek operations

#### 4. User Balances

Individual balance tracking without token custody:

- **Base Balance**: User's base token balance in the market
- **Quote Balance**: User's quote token balance in the market
- **Per-Market**: Separate balance account for each market
- **No Token Holding**: Balances are accounting records, not actual token accounts

#### 5. Event Queue & Processing

**New Feature**: Asynchronous balance update system:

- **Event Queue**: Circular buffer storing fill events for deferred processing
- **Two-Phase Updates**: Taker balances updated immediately, maker balances queued
- **Sequential Processing**: Events processed in strict FIFO order
- **Consume Events**: Dedicated instruction to process queued maker balance updates

### Key Design Principles

1. **High Performance**: Zero-copy heap orderbooks for optimal memory usage and speed
2. **Centralized Liquidity**: All tokens held in market vaults with separate balance tracking
3. **Price-Time Priority**: Orders matched by best price first, then earliest timestamp
4. **Event-Driven Architecture**: Comprehensive event emission with asynchronous processing
5. **Professional Order Management**: Complete time-in-force support with GTC, IOC, and FOK order types

### 📋 Order Management Features

#### Time-in-Force Support

Advanced order execution control with three professional time-in-force types:

- **GTC (Good-Till-Cancelled)**: Default order type that remains active in the orderbook until explicitly cancelled or filled
- **IOC (Immediate-Or-Cancel)**: Executes immediately against available liquidity; any unfilled portion is automatically cancelled
- **FOK (Fill-Or-Kill)**: Must be filled completely and immediately, or the entire order is rejected

#### Order Lifecycle

1. **Placement**: Orders are validated, matched against existing liquidity, and processed according to time-in-force rules
2. **Matching**: Automatic execution using price-time priority matching algorithm
3. **Settlement**: Two-phase balance updates (immediate taker, queued maker)
4. **Management**: Orders can be cancelled at any time, with automatic balance restoration

### Account Structure

- **Market**: Main market configuration and state
- **BidSide/AskSide**: Zero-copy heap orderbook accounts for bids and asks
- **EventQueue**: Zero-copy circular buffer for fill events
- **UserBalance**: Individual user balance tracking per market
- **Token Vaults**: PDA-controlled token accounts holding all market liquidity

## 🚀 Quick Start

### Prerequisites

- Rust rustc 1.91.0-nightly (898aff704 2025-08-14)
- solana-cli 2.1.21
- Anchor 0.31.1
- Node.js 22+
- Yarn

### Build

```bash
# Build the Solana program
cargo build-sbf

# gen idl
anchor build
```

### Test

```bash
# Run all tests
cargo test-sbf

# Run specific test suites
cargo test-sbf test_vault_workflow          # Vault operations
cargo test-sbf test_orderbook_basic_matching # Order matching
cargo test-sbf test_consume_events_basic     # Event queue processing
cargo test-sbf test_time_in_force            # Time-in-force order types

# Run with verbose output
cargo test-sbf test_orderbook_basic_matching -- --nocapture
```

## 📖 API Reference

### Core Instructions

#### 1. Initialize Market

Creates a new trading market with base/quote token pair and initializes orderbooks and event queue.

```rust
pub fn initialize(
    ctx: Context<Initialize>,
    params: InitializeParams
) -> Result<()>

// Parameters
struct InitializeParams {
    base_mint: Pubkey,      // Base token mint
    quote_mint: Pubkey,     // Quote token mint
    base_lot_size: u64,     // Minimum base token unit (e.g., 1_000_000 for 6 decimals)
    quote_tick_size: u64,   // Minimum quote price unit (e.g., 1_000 for 0.001)
}
```

#### 2. Deposit Tokens

Deposits tokens into the market vault and updates user balance.

```rust
pub fn deposit(
    ctx: Context<Deposit>,
    params: DepositParams
) -> Result<()>

// Parameters
struct DepositParams {
    amount: u64,            // Amount to deposit (in token's native units)
}
```

#### 3. Place Limit Order

Places a limit order with automatic matching, time-in-force handling, and event queue integration.

```rust
pub fn place_limit_order(
    ctx: Context<PlaceLimitOrder>,
    params: PlaceLimitOrderParams
) -> Result<()>

// Parameters
struct PlaceLimitOrderParams {
    side: Side,                 // Side::Bid (buy) or Side::Ask (sell)
    price: u64,                 // Price in quote_tick_size units
    quantity: u64,              // Quantity in base_lot_size units
    time_in_force: TimeInForce, // Order time-in-force type
}

// Time-in-Force Types
enum TimeInForce {
    GTC = 0, // Good-Till-Cancelled: Order remains active until explicitly cancelled
    IOC = 1, // Immediate-Or-Cancel: Execute immediately, cancel any unfilled portion
    FOK = 2, // Fill-Or-Kill: Either fill the entire order immediately or cancel it completely
}
```

**Behavior**:

- **GTC Orders**: Taker balances are updated immediately upon matching, maker balance updates are queued in the event queue, remaining order quantity is added to the appropriate orderbook
- **IOC Orders**: Execute immediately against available liquidity, any unfilled portion is cancelled (no resting orders created)
- **FOK Orders**: Either fill the entire order immediately or reject the transaction with `FillOrKillNotFilled` error

#### 4. Consume Events

**New Instruction**: Processes queued fill events to update maker balances.

```rust
pub fn consume_events(
    ctx: Context<ConsumeEvents>,
    params: ConsumeEventsParams
) -> Result<()>

// Parameters
struct ConsumeEventsParams {
    limit: u8,             // Maximum number of events to process
}
```

**Behavior**:

- Processes events sequentially in FIFO order
- Updates maker balances based on filled orders
- Stops processing if a maker account is not provided
- Removes processed events from the queue

#### 5. Cancel Order

Cancels an existing limit order and returns reserved funds.

```rust
pub fn cancel_order(
    ctx: Context<CancelOrder>,
    params: CancelOrderParams
) -> Result<()>

// Parameters
struct CancelOrderParams {
    order_id: u64,         // Order ID to cancel
    side: Side,            // Which orderbook to search (Bid/Ask)
}
```

#### 6. Withdraw Tokens

Withdraws tokens from market vault to user's token account.

```rust
pub fn withdraw(
    ctx: Context<Withdraw>,
    params: WithdrawParams
) -> Result<()>

// Parameters
struct WithdrawParams {
    amount: u64,           // Amount to withdraw
}
```

### Events

The program emits comprehensive events for all operations:

```rust
// Order placement
#[event]
pub struct OrderPlaced {
    pub order_id: u64,
    pub owner: Pubkey,
    pub market: Pubkey,
    pub side: Side,
    pub price: u64,
    pub quantity: u64,
    pub timestamp: i64,
}

// Order matching/execution
#[event]
pub struct OrderFilled {
    pub maker_order_id: u64,
    pub taker_order_id: u64,
    pub market: Pubkey,
    pub price: u64,
    pub quantity: u64,
    pub maker_owner: Pubkey,
    pub taker_owner: Pubkey,
    pub taker_side: Side,
}

// Order cancellation
#[event]
pub struct OrderCancelled {
    pub order_id: u64,
    pub owner: Pubkey,
    pub market: Pubkey,
    pub side: Side,
    pub remaining_quantity: u64,
}

// Market initialization
#[event]
pub struct MarketInitialized {
    pub market: Pubkey,
    pub authority: Pubkey,
    pub base_mint: Pubkey,
    pub quote_mint: Pubkey,
    pub base_lot_size: u64,
    pub quote_tick_size: u64,
}
```

## 💡 Example Usage

### Workflow Pseudocode

```
1. CREATE MARKET & INFRASTRUCTURE
   ├── create bids orderbook (zero-copy heap)
   ├── create asks orderbook (zero-copy heap)
   ├── create event_queue (circular buffer)
   └── initialize market (base_mint, quote_mint, lot_sizes)

2. DEPOSIT FUNDS
   ├── user_balance ← deposit(base_tokens)
   └── user_balance ← deposit(quote_tokens)

3. PLACE ORDERS
   ├── place_limit_order(side, price, quantity, time_in_force)
   ├── automatic matching against existing orders
   ├── taker balance updated immediately
   └── maker balance updates queued in event_queue

4. CONSUME EVENTS (Process Queued Updates)
   ├── consume_events(limit)
   ├── process fill events in FIFO order
   └── update maker balances from event_queue

5. WITHDRAW FUNDS
   ├── withdraw(base_amount) → user_token_account
   └── withdraw(quote_amount) → user_token_account

6. CLEANUP (Optional)
   └── close_user_balance (when empty)
```

## 🧪 Testing

### Test Structure

- **Unit Tests**: Located in `programs/clob/tests/cases/`
  - `test_vault_workflow.rs`: Vault operations and balance management
  - `test_orderbook_workflow.rs`: Order placement, matching, and cancellation
  - `test_consume_events.rs`: Event queue and balance update processing
  - `test_time_in_force.rs`: Time-in-force order types (GTC, IOC, FOK)

### Running Tests

```bash
# Run all tests with output
cargo test-sbf -- --nocapture

# Test specific functionality
cargo test-sbf test_orderbook_basic_matching -- --nocapture
cargo test-sbf test_consume_events_basic -- --nocapture

# Test vault operations
cargo test-sbf test_vault_workflow -- --nocapture
```

### Test Scenarios Covered

1. **Basic Operations**

   - Market initialization with orderbooks and event queue
   - Token deposits and withdrawals
   - User balance management

2. **Order Matching**

   - Limit order placement with zero-copy heap orderbooks
   - Automatic order matching with price-time priority
   - Partial fills and remaining quantity handling
   - Immediate taker balance updates

3. **Event Processing**

   - Fill event creation and queuing
   - Sequential event consumption
   - Maker balance updates via consume_events
   - Event queue management

4. **Order Management**

   - Order cancellation with balance restoration
   - Balance reservation and release
   - Comprehensive error handling

5. **Time-in-Force Orders**
   - GTC orders: remain active until cancelled
   - IOC orders: immediate execution with unfilled cancellation
   - FOK orders: complete fill or rejection
   - Mixed scenarios with different order types

## 🔧 Configuration

### Market Parameters

- **Base Lot Size**: Minimum tradeable unit for base token
- **Quote Tick Size**: Minimum price increment
- **Event Queue Size**: 256 events (configurable via MAX_EVENTS)

### Program Configuration

```toml
# Anchor.toml
[programs.localnet]
clob = "FpTyzdMqQS4NWM149ryMWq74waAoHXMBpJnXb4yUNV1F"

[registry]
url = "https://api.apr.dev"

[provider]
cluster = "Localnet"
wallet = "~/.config/solana/id.json"
```

## 🚀 Performance Improvements

### Zero-Copy Heap Orderbooks

**Before (Vec-based)**:

- Serialization overhead on every access
- O(n) insertions due to sorting requirements
- Limited capacity (50 orders per side)

**After (Heap-based)**:

- Zero-copy direct memory access
- O(log n) insertions with automatic heap ordering
- Larger capacity with efficient memory usage
- Better cache locality for high-frequency operations

### Event Queue Architecture

**Benefits**:

- **Scalability**: Defers expensive balance calculations
- **Consistency**: Processes events in strict order
- **Flexibility**: Allows batched event processing
- **Performance**: Reduces transaction complexity for order matching

## 🚧 Current Features & Roadmap

### ✅ Implemented Features

- Zero-copy heap-based orderbooks for optimal performance
- Event queue with sequential processing
- Two-phase balance update system
- Price-time priority matching
- Time-in-Force Support: GTC, IOC, and FOK order types
- Comprehensive event emission
- Full test coverage

### 🔄 Potential Future Enhancements

- Enhanced error handling and recovery
- Performance metrics and monitoring
