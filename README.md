# Solana Orderbook DEX

A decentralized limit order book implementation built on Solana using the Anchor framework. This DEX features centralized liquidity, efficient order matching, and comprehensive event emission for real-time market data.

## 🏗️ High-Level Architecture

### Core Components

The DEX consists of three main architectural layers:

#### 1. Market State
The central `Market` account contains:
- **Base Mint**: Address of the base token (e.g., SOL)
- **Quote Mint**: Address of the quote token (e.g., USDC)  
- **Lot Sizes**: Minimum tradeable units for price and quantity
- **Next Order ID**: Global counter for unique order identification

#### 2. Token Vaults
Centralized liquidity storage:
- **Base Vault**: PDA-controlled token account holding all base tokens
- **Quote Vault**: PDA-controlled token account holding all quote tokens
- **Centralized Model**: All user funds pooled for efficient matching

#### 3. Order Books
Separate accounts for buy and sell orders:
- **Bids Book**: Contains all buy orders (Side::Bid)
- **Asks Book**: Contains all sell orders (Side::Ask)
- **Price-Time Priority**: Orders sorted by best price, then earliest timestamp
- **Vec Storage**: Currently implemented with Vec<Order> (max 50 orders per side)

#### 4. User Balances  
Individual balance tracking without token custody:
- **Base Balance**: User's base token balance in the market
- **Quote Balance**: User's quote token balance in the market
- **Per-Market**: Separate balance account for each market
- **No Token Holding**: Balances are accounting records, not actual token accounts

### Key Design Principles

1. **Centralized Liquidity**: All tokens are held in market vaults, with user balances tracked separately
2. **Price-Time Priority**: Orders are matched based on best price first, then earliest timestamp
3. **Event-Driven**: All operations emit comprehensive events for real-time market data
4. **Anchor Framework**: Type-safe, modern Solana development with automatic serialization
5. **Modular Architecture**: Pluggable orderbook implementations (currently Vec-based)

### Account Structure

- **Market**: Main market configuration and state
- **BookSide**: Separate accounts for bids and asks orderbooks
- **UserBalance**: Individual user balance tracking per market
- **Token Vaults**: PDA-controlled token accounts holding all market liquidity

## 🚀 Quick Start

### Prerequisites

- Rust 1.86.0 (Other versions should also work, but haven't been tested.)
- Solana CLI 2.2.0
- Anchor 0.31.1
- Node.js 22+ (for tests)
- Yarn

### Installation

```bash
# Install dependencies
yarn install

# Install Solana tools
sh -c "$(curl -sSfL https://release.solana.com/v2.2.0/install)"
cargo install --git https://github.com/coral-xyz/anchor avm --locked --force
avm install latest
avm use latest
```

### Build

```bash
# Build the Solana program
anchor build

# Build for testing
cargo build-sbf
```

### Test

```bash
# Run all tests
anchor test

# Run specific test suites
cargo test-sbf test_vault_workflow          # Vault operations
cargo test-sbf test_orderbook               # Orderbook functionality
cargo test-sbf test_orderbook_basic_matching # Order matching

# Run with verbose output
cargo test-sbf test_orderbook_basic_matching -- --nocapture
```

## 📖 API Reference

### Core Instructions

#### 1. Initialize Market

Creates a new trading market with base/quote token pair.

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

Places a limit order in the orderbook with automatic matching.

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

#### 4. Cancel Order

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

#### 5. Withdraw Tokens

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
```

## 💡 Example Usage

Here's a complete example of typical DEX operations:

```typescript
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Clob } from "../target/types/clob";

// 1. Initialize market
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
    bidsBook: bidsBookPda,
    asksBook: asksBookPda,
  })
  .rpc();

// 2. User deposits base tokens
await program.methods
  .deposit({ amount: new anchor.BN(100_000_000) }) // 100 tokens
  .accounts({
    user: alice.publicKey,
    market: marketPda,
    userBalance: aliceBalancePda,
    userTokenAccount: aliceBaseAccount,
    vaultTokenAccount: baseVaultPda,
    mint: baseMint.publicKey,
  })
  .signers([alice])
  .rpc();

// 3. Alice places sell order (ask)
await program.methods
  .placeLimitOrder({
    side: { ask: {} },
    price: new anchor.BN(5), // Price 5 (in tick units)
    quantity: new anchor.BN(10), // Quantity 10 (in lot units)
  })
  .accounts({
    market: marketPda,
    bidsBook: bidsBookPda,
    asksBook: asksBookPda,
    userBalance: aliceBalancePda,
    baseVault: baseVaultPda,
    quoteVault: quoteVaultPda,
    user: alice.publicKey,
  })
  .signers([alice])
  .rpc();

// 4. Bob places matching buy order (bid)
await program.methods
  .placeLimitOrder({
    side: { bid: {} },
    price: new anchor.BN(5), // Same price - will match
    quantity: new anchor.BN(5), // Partial fill
  })
  .accounts({
    market: marketPda,
    bidsBook: bidsBookPda,
    asksBook: asksBookPda,
    userBalance: bobBalancePda,
    baseVault: baseVaultPda,
    quoteVault: quoteVaultPda,
    user: bob.publicKey,
  })
  .signers([bob])
  .rpc();

// 5. Alice cancels remaining order
await program.methods
  .cancelOrder({
    orderId: new anchor.BN(1),
    side: { ask: {} },
  })
  .accounts({
    market: marketPda,
    bidsBook: bidsBookPda,
    asksBook: asksBookPda,
    userBalance: aliceBalancePda,
    user: alice.publicKey,
  })
  .signers([alice])
  .rpc();
```

## 🧪 Testing

### Test Structure

- **Unit Tests**: Located in `programs/clob/tests/`
  - `test_vault_workflow.rs`: Vault operations and balance management
  - `test_orderbook_workflow.rs`: Order placement, matching, and cancellation

### Running Tests

```bash
# Run all tests with output
cargo test-sbf -- --nocapture

# Test specific functionality
cargo test-sbf test_orderbook_basic_matching -- --nocapture
cargo test-sbf test_partial_fills_and_price_time_priority -- --nocapture

# Test vault operations
cargo test-sbf test_vault_workflow -- --nocapture
```

### Test Scenarios Covered

1. **Basic Operations**

   - Market initialization
   - Token deposits and withdrawals
   - User balance management

2. **Order Matching**

   - Limit order placement
   - Automatic order matching
   - Partial fills
   - Price-time priority

3. **Order Management**

   - Order cancellation
   - Balance reservation/release
   - Error handling

4. **Event Emission**
   - All operations emit appropriate events
   - Event data validation

## 🔧 Configuration

### Market Parameters

- **Base Lot Size**: Minimum tradeable unit for base token
- **Quote Tick Size**: Minimum price increment
- **Max Orders**: Currently 50 orders per orderbook side

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

## 🚧 Current Limitations

- **Vec-based Orderbook**: Simple but not optimal for high-frequency trading
- **Order Limit**: Maximum 50 orders per side (configurable)
- **No Market Orders**: Only limit orders currently supported
- **Basic Order Types**: No IOC, FOK, or Post-Only orders yet

---

**Program ID**: `FpTyzdMqQS4NWM149ryMWq74waAoHXMBpJnXb4yUNV1F`

For more detailed information, check the inline documentation in the source code and the comprehensive test suite.
