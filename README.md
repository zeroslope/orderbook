# Solana CLOB (Central Limit Order Book) DEX

A high-performance decentralized limit order book implementation built on Solana using the Anchor framework. This CLOB features centralized liquidity, efficient order matching with zero-copy heap-based orderbooks, and comprehensive event processing for real-time market data.

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
### Account Structure

- **Market**: Main market configuration and state
- **BidSide/AskSide**: Zero-copy heap orderbook accounts for bids and asks
- **EventQueue**: Zero-copy circular buffer for fill events
- **UserBalance**: Individual user balance tracking per market
- **Token Vaults**: PDA-controlled token accounts holding all market liquidity

## 🚀 Quick Start

### Prerequisites

- Rust 1.86.0 (Other versions should also work, but haven't been tested.)
- Solana CLI 2.2.0
- Anchor 0.31.1
- Node.js 22+
- Yarn
### Build

```bash
# Build the Solana program
cargo build-sbf

# Alternative: Build with Anchor
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

Places a limit order with automatic matching and event queue integration.

```rust
pub fn place_limit_order(
    ctx: Context<PlaceLimitOrder>,
    params: PlaceLimitOrderParams
) -> Result<()>

// Parameters
struct PlaceLimitOrderParams {
    side: Side,            // Side::Bid (buy) or Side::Ask (sell)
    price: u64,            // Price in quote_tick_size units
    quantity: u64,         // Quantity in base_lot_size units
}
```

**Behavior**: 
- Taker balances are updated immediately upon matching
- Maker balance updates are queued in the event queue
- Remaining order quantity is added to the appropriate orderbook

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

Here's a complete example showing the two-phase balance update system:

```typescript
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Clob } from "../target/types/clob";

// 1. Initialize market with orderbooks and event queue
const initializeParams = {
  baseMint: baseMint.publicKey,
  quoteMint: quoteMint.publicKey,
  baseLotSize: new anchor.BN(1_000_000), // 1.0 base token
  quoteTickSize: new anchor.BN(1_000), // 0.001 quote token
};

await program.methods
  .initialize(initializeParams)
  .accounts({
    authority: authority.publicKey,
    market: marketPda,
    baseVault: baseVaultPda,
    quoteVault: quoteVaultPda,
    baseMint: baseMint.publicKey,
    quoteMint: quoteMint.publicKey,
    bids: bidsPda,           // Zero-copy heap orderbook
    asks: asksPda,          // Zero-copy heap orderbook  
    eventQueue: eventQueuePda, // Event queue for deferred updates
  })
  .rpc();

// 2. Alice places sell order (maker)
await program.methods
  .placeLimitOrder({
    side: { ask: {} },
    price: new anchor.BN(2000),
    quantity: new anchor.BN(5),
  })
  .accounts({
    market: marketPda,
    bids: bidsPda,
    asks: asksPda,
    eventQueue: eventQueuePda,
    userBalance: aliceBalancePda,
    baseVault: baseVaultPda,
    quoteVault: quoteVaultPda,
    user: alice.publicKey,
  })
  .signers([alice])
  .rpc();

// 3. Bob places matching buy order (taker)
// Taker balance updated immediately, maker balance queued
await program.methods
  .placeLimitOrder({
    side: { bid: {} },
    price: new anchor.BN(2000),
    quantity: new anchor.BN(5),
  })
  .accounts({
    market: marketPda,
    bids: bidsPda,
    asks: asksPda,
    eventQueue: eventQueuePda,
    userBalance: bobBalancePda,
    baseVault: baseVaultPda,
    quoteVault: quoteVaultPda,
    user: bob.publicKey,
  })
  .signers([bob])
  .rpc();

// 4. Process queued events to update Alice's balance
await program.methods
  .consumeEvents({ limit: 10 })
  .accounts({
    market: marketPda,
    eventQueue: eventQueuePda,
  })
  .remainingAccounts([
    {
      pubkey: aliceBalancePda,
      isWritable: true,
      isSigner: false,
    }
  ])
  .rpc();
```

## 🧪 Testing

### Test Structure

- **Unit Tests**: Located in `programs/clob/tests/cases/`
  - `test_vault_workflow.rs`: Vault operations and balance management
  - `test_orderbook_workflow.rs`: Order placement, matching, and cancellation
  - `test_consume_events.rs`: Event queue and balance update processing

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
- Comprehensive event emission
- Full test coverage

### 🔄 Potential Future Enhancements

- Market orders and advanced order types (IOC, FOK, Post-Only)
- Cross-program invocation support
- Enhanced error handling and recovery
- Performance metrics and monitoring

---

**Program ID**: `FpTyzdMqQS4NWM149ryMWq74waAoHXMBpJnXb4yUNV1F`

For more detailed information, check the inline documentation in the source code and the comprehensive test suite.