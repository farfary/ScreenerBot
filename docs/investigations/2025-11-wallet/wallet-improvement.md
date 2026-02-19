# NFT Detection & Separation Investigation

**Date:** December 3, 2025  
**Issue:** NFTs appear in wallet token holdings without proper identification/separation  
**Example NFT:** `CmpPkVrJrvZeZZTz3W3u8pbrQbmzuiCbAxFcWKo8Ux1G`

---

## Executive Summary

**CRITICAL FINDING:** The bot has an NFT detection function (`extract_nft_mint_if_valid`) that is **NOT being used**. NFTs are currently treated as regular tokens, causing UI confusion and incorrect balance calculations.

**ROOT CAUSE:**

1. Wallet service fetches ALL token accounts (including NFTs)
2. No filtering happens during collection
3. NFT detection function exists in `rpc.rs` but is **never called**
4. Database and UI have no NFT separation logic

**IMPACT:**

- NFTs show in "Token Holdings" tab
- Balance displays are misleading (showing "1.0" for NFTs)
- No metadata/images for NFTs (they don't have market data)
- User confusion about what they own

---

## 1. NFT Characteristics on Solana

### Standard NFT Definition

On Solana, an NFT is identified by:

1. **Supply:** Total supply = 1 (not divisible)
2. **Decimals:** 0 (no fractional ownership)
3. **Amount:** User owns exactly 1 token
4. **Metadata:** Metaplex Token Metadata Program account exists

### Technical Implementation

```rust
// From src/rpc.rs line 5885
fn extract_nft_mint_if_valid(account: &serde_json::Value) -> Option<String> {
    // ...

    // Check decimals = 0
    let decimals = token_amount.get("decimals")?.as_u64()?;
    if decimals != 0 {
        return None;
    }

    // Check amount = 1 (use uiAmount which is already adjusted for decimals)
    let ui_amount = token_amount.get("uiAmount")?.as_f64()?;
    if (ui_amount - 1.0).abs() > 0.0001 {
        return None;
    }

    Some(mint.to_string())
}
```

**This function exists but is NEVER called!**

---

## 2. Current Wallet Token Collection Flow

### Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────┐
│ Wallet Service (src/wallet.rs::collect_wallet_snapshot)    │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ RPC Client (src/rpc.rs::get_all_token_accounts)            │
│  • Calls: getTokenAccountsByOwner (SPL Token)               │
│  • Calls: getTokenAccountsByOwner (Token-2022)              │
│  • Encoding: "jsonParsed"                                   │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ extract_token_account_info() - line 5860                    │
│  ✓ Extracts: pubkey, mint, balance, is_token_2022          │
│  ❌ NO NFT DETECTION - Returns ALL as TokenAccountInfo     │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ Returns: Vec<TokenAccountInfo>                              │
│  • Contains BOTH tokens AND NFTs mixed together             │
│  • No is_nft flag                                           │
│  • No separation logic                                      │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ Wallet Database (wallet.db::token_balances)                │
│  • Stores NFTs as tokens                                    │
│  • decimals = NULL                                          │
│  • balance = 1                                              │
│  • balance_ui = 1.0                                         │
└───────────────────────┬─────────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────────┐
│ Dashboard API (GET /api/wallet/dashboard)                  │
│  • Fetches token metadata via enrich_token_overview()      │
│  • NFTs have NO market data → no symbol/name/logo          │
│  • Shows as "CmpP..." with generic icon                    │
└─────────────────────────────────────────────────────────────┘
```

### Code Evidence

**Location:** `src/wallet.rs` line 2347

```rust
async fn collect_wallet_snapshot() -> Result<WalletSnapshot, String> {
    // ...

    // Get all token accounts
    let token_accounts = rpc_client
        .get_all_token_accounts(&wallet_address)
        .await
        .map_err(|e| format!("Failed to get token accounts: {}", e))?;

    // Convert to TokenBalance format
    let mut token_balances = Vec::new();
    for account_info in &token_accounts {
        // Skip accounts with zero balance
        if account_info.balance == 0 {
            continue;
        }

        // ❌ NO NFT CHECK HERE!

        token_balances.push(TokenBalance {
            id: None,
            snapshot_id: None,
            mint: account_info.mint.clone(),
            balance: account_info.balance,
            balance_ui,
            decimals: crate::tokens::get_cached_decimals(&account_info.mint),
            is_token_2022: account_info.is_token_2022,
        });
    }
    // ...
}
```

---

## 3. Database Analysis

### Current wallet.db Schema

```sql
CREATE TABLE token_balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    balance INTEGER NOT NULL,
    balance_ui REAL NOT NULL,
    decimals INTEGER,                          -- ❌ NULL for NFTs
    is_token_2022 BOOLEAN NOT NULL DEFAULT false,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (snapshot_id) REFERENCES wallet_snapshots(id) ON DELETE CASCADE
);
```

**Missing Fields:**

- `is_nft BOOLEAN` - No NFT flag
- `nft_name TEXT` - No NFT name storage
- `nft_image_url TEXT` - No NFT image storage
- `nft_collection TEXT` - No collection tracking

### Current Data for Example NFT

```sql
SELECT mint, balance, balance_ui, decimals, is_token_2022
FROM token_balances
WHERE mint = 'CmpPkVrJrvZeZZTz3W3u8pbrQbmzuiCbAxFcWKo8Ux1G';

-- Result:
-- CmpPkVrJrvZeZZTz3W3u8pbrQbmzuiCbAxFcWKo8Ux1G|1|1.0||0
```

**Issues:**

- `decimals = NULL` (should be 0 for NFT)
- No indication this is an NFT
- Mixed with fungible tokens in queries

---

## 4. Why NFTs Don't Show Metadata

### Problem Chain

```
1. NFT in wallet.db (CmpPkVrJrvZeZZTz3W3u8pbrQbmzuiCbAxFcWKo8Ux1G)
        ↓
2. enrich_token_overview() tries to fetch from tokens database
        ↓
3. get_full_token_async(mint) → Returns None (NFT not in database)
        ↓
4. Token discovery filters NFTs (they have no DexScreener pairs)
        ↓
5. Frontend shows: symbol="CmpP...", name=None, image_url=None
        ↓
6. User sees: Generic coin icon + truncated address
```

### Why NFTs Don't Get Discovered

From `src/tokens/discovery.rs`:

- **DexScreener:** Requires trading pairs (NFTs have none)
- **GeckoTerminal:** Requires liquidity pools (NFTs have none)
- **Rugcheck:** Focuses on tokens (not NFTs)
- **Jupiter:** Requires tradeable tokens (NFTs aren't traded this way)

**NFTs are fundamentally different:**

- No price/liquidity/volume data
- Metadata from Metaplex, not market APIs
- Identified by collection + unique attributes
- Display requires image URL from metadata JSON

---

## 5. NFT Metadata Structure (Metaplex Standard)

### On-Chain Metadata Account

```rust
// Metaplex Token Metadata PDA
// Seeds: ["metadata", METAPLEX_PROGRAM_ID, mint_pubkey]
pub struct Metadata {
    pub key: Key,                    // MetadataV1 discriminator
    pub update_authority: Pubkey,    // Can update metadata
    pub mint: Pubkey,                // NFT mint address
    pub data: Data,                  // Name, symbol, URI
    pub primary_sale_happened: bool,
    pub is_mutable: bool,
    pub edition_nonce: Option<u8>,
    pub token_standard: Option<TokenStandard>, // NonFungible, etc.
    pub collection: Option<Collection>,        // Collection info
    pub uses: Option<Uses>,
}

pub struct Data {
    pub name: String,               // "Cool NFT #1234"
    pub symbol: String,             // "COOL"
    pub uri: String,                // "https://arweave.net/..."
    pub seller_fee_basis_points: u16,
    pub creators: Option<Vec<Creator>>,
}
```

### Off-Chain Metadata (JSON at URI)

```json
{
  "name": "Cool NFT #1234",
  "symbol": "COOL",
  "description": "A cool NFT from the collection",
  "image": "https://arweave.net/xyz.png",
  "attributes": [
    { "trait_type": "Background", "value": "Blue" },
    { "trait_type": "Eyes", "value": "Laser" }
  ],
  "properties": {
    "files": [
      { "uri": "https://arweave.net/xyz.png", "type": "image/png" }
    ],
    "category": "image",
    "creators": [...]
  }
}
```

---

## 6. Systematic Solution Architecture

### Design Principles

1. **Detect at Source:** Identify NFTs during RPC fetch
2. **Separate Storage:** NFTs in separate table(s)
3. **Different Enrichment:** Fetch Metaplex metadata, not market data
4. **UI Separation:** Dedicated NFT tab in dashboard
5. **Backward Compatible:** Don't break existing token logic

---

### Component 1: Enhanced RPC Token Account Parsing

**Location:** `src/rpc.rs`

**New Structure:**

```rust
#[derive(Debug)]
pub enum TokenAccountType {
    FungibleToken(TokenAccountInfo),
    NonFungibleToken(NftAccountInfo),
}

#[derive(Debug)]
pub struct NftAccountInfo {
    pub account: String,
    pub mint: String,
    pub is_token_2022: bool,
}

// Modified return type
pub async fn get_all_token_accounts_v2(
    &self,
    wallet_address: &str,
) -> Result<(Vec<TokenAccountInfo>, Vec<NftAccountInfo>), ScreenerBotError>
```

**Implementation:**

```rust
// In get_all_token_accounts loop:
for account in accounts {
    // Try NFT extraction first
    if let Some(nft_mint) = extract_nft_mint_if_valid(account) {
        nft_accounts.push(NftAccountInfo {
            account: extract_pubkey(account),
            mint: nft_mint,
            is_token_2022,
        });
    } else if let Some(token_info) = extract_token_account_info(account, is_token_2022) {
        token_accounts.push(token_info);
    }
}

return Ok((token_accounts, nft_accounts));
```

---

### Component 2: Wallet Database Schema Extension

**New Table:**

```sql
CREATE TABLE IF NOT EXISTS nft_balances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_id INTEGER NOT NULL,
    mint TEXT NOT NULL,
    account_address TEXT NOT NULL,
    name TEXT,
    symbol TEXT,
    image_url TEXT,
    metadata_uri TEXT,
    collection_mint TEXT,
    collection_name TEXT,
    is_token_2022 BOOLEAN NOT NULL DEFAULT false,
    metadata_fetched_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (snapshot_id) REFERENCES wallet_snapshots(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_nft_balances_snapshot_id ON nft_balances(snapshot_id);
CREATE INDEX IF NOT EXISTS idx_nft_balances_mint ON nft_balances(mint);
CREATE INDEX IF NOT EXISTS idx_nft_balances_collection ON nft_balances(collection_mint);
```

**Modified Snapshot Structure:**

```rust
pub struct WalletSnapshot {
    pub id: Option<i64>,
    pub wallet_address: String,
    pub snapshot_time: DateTime<Utc>,
    pub sol_balance: f64,
    pub sol_balance_lamports: u64,
    pub total_tokens_count: u32,
    pub total_nfts_count: u32,           // NEW
    pub token_balances: Vec<TokenBalance>,
    pub nft_balances: Vec<NftBalance>,   // NEW
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftBalance {
    pub id: Option<i64>,
    pub snapshot_id: Option<i64>,
    pub mint: String,
    pub account_address: String,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub image_url: Option<String>,
    pub metadata_uri: Option<String>,
    pub collection_mint: Option<String>,
    pub collection_name: Option<String>,
    pub is_token_2022: bool,
    pub metadata_fetched_at: Option<DateTime<Utc>>,
}
```

---

### Component 3: NFT Metadata Fetcher Module

**New Module:** `src/nfts/metadata.rs`

```rust
use crate::rpc::get_rpc_client;
use crate::constants::METAPLEX_PROGRAM_ID;
use solana_sdk::pubkey::Pubkey;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftMetadata {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub image_url: Option<String>,
    pub description: Option<String>,
    pub attributes: Vec<NftAttribute>,
    pub collection: Option<NftCollection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftAttribute {
    pub trait_type: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NftCollection {
    pub name: String,
    pub family: Option<String>,
}

/// Fetch NFT metadata from Metaplex + off-chain JSON
pub async fn fetch_nft_metadata(mint: &str) -> Result<NftMetadata, String> {
    // 1. Derive Metaplex metadata PDA
    let metadata_pda = derive_metadata_pda(mint)?;

    // 2. Fetch on-chain metadata account
    let onchain_data = fetch_metaplex_account(&metadata_pda).await?;

    // 3. Parse Metaplex metadata structure
    let parsed = parse_metaplex_metadata(&onchain_data)?;

    // 4. Fetch off-chain JSON from URI
    let offchain_data = fetch_json_metadata(&parsed.uri).await?;

    // 5. Combine into unified structure
    Ok(NftMetadata {
        name: parsed.name,
        symbol: parsed.symbol,
        uri: parsed.uri,
        image_url: offchain_data.image,
        description: offchain_data.description,
        attributes: offchain_data.attributes.unwrap_or_default(),
        collection: offchain_data.collection,
    })
}

fn derive_metadata_pda(mint: &str) -> Result<Pubkey, String> {
    let mint_pubkey = Pubkey::from_str(mint)
        .map_err(|e| format!("Invalid mint: {}", e))?;
    let metaplex_program = Pubkey::from_str(METAPLEX_PROGRAM_ID)
        .map_err(|e| format!("Invalid program ID: {}", e))?;

    let seeds = &[
        b"metadata",
        metaplex_program.as_ref(),
        mint_pubkey.as_ref(),
    ];

    let (pda, _bump) = Pubkey::find_program_address(seeds, &metaplex_program);
    Ok(pda)
}

async fn fetch_metaplex_account(pda: &Pubkey) -> Result<Vec<u8>, String> {
    let rpc = get_rpc_client();
    let account = rpc.get_account(pda).await
        .map_err(|e| format!("Failed to fetch metadata account: {}", e))?;
    Ok(account.data)
}

fn parse_metaplex_metadata(data: &[u8]) -> Result<OnChainMetadata, String> {
    // Simplified - use mpl-token-metadata crate for full implementation
    // Parse borsh-serialized Metadata account
    todo!("Implement with mpl-token-metadata crate")
}

async fn fetch_json_metadata(uri: &str) -> Result<OffChainMetadata, String> {
    let client = reqwest::Client::new();
    let response = client.get(uri)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch JSON: {}", e))?;

    let json: OffChainMetadata = response.json().await
        .map_err(|e| format!("Failed to parse JSON: {}", e))?;

    Ok(json)
}
```

---

### Component 4: Modified Wallet Collection Flow

**Location:** `src/wallet.rs::collect_wallet_snapshot()`

```rust
async fn collect_wallet_snapshot() -> Result<WalletSnapshot, String> {
    let wallet_address = get_wallet_address()?;
    let rpc_client = get_rpc_client();
    let snapshot_time = Utc::now();

    // Get SOL balance
    let sol_balance = rpc_client.get_sol_balance(&wallet_address).await?;
    let sol_balance_lamports = sol_to_lamports(sol_balance);

    // Get token accounts WITH NFT SEPARATION
    let (token_accounts, nft_accounts) = rpc_client
        .get_all_token_accounts_v2(&wallet_address)
        .await
        .map_err(|e| format!("Failed to get accounts: {}", e))?;

    // Process fungible tokens
    let mut token_balances = Vec::new();
    for account_info in &token_accounts {
        if account_info.balance == 0 {
            continue;
        }

        let decimals = crate::tokens::decimals::get(&account_info.mint).await;
        let balance_ui = if let Some(d) = decimals {
            (account_info.balance as f64) / (10_f64).powi(d as i32)
        } else {
            account_info.balance as f64
        };

        token_balances.push(TokenBalance {
            id: None,
            snapshot_id: None,
            mint: account_info.mint.clone(),
            balance: account_info.balance,
            balance_ui,
            decimals,
            is_token_2022: account_info.is_token_2022,
        });
    }

    // Process NFTs
    let mut nft_balances = Vec::new();
    for nft_info in &nft_accounts {
        // Fetch NFT metadata asynchronously
        let metadata = crate::nfts::fetch_nft_metadata(&nft_info.mint)
            .await
            .ok(); // Don't fail snapshot if metadata fetch fails

        nft_balances.push(NftBalance {
            id: None,
            snapshot_id: None,
            mint: nft_info.mint.clone(),
            account_address: nft_info.account.clone(),
            name: metadata.as_ref().map(|m| m.name.clone()),
            symbol: metadata.as_ref().map(|m| m.symbol.clone()),
            image_url: metadata.as_ref().and_then(|m| m.image_url.clone()),
            metadata_uri: metadata.as_ref().map(|m| m.uri.clone()),
            collection_mint: metadata.as_ref()
                .and_then(|m| m.collection.as_ref())
                .map(|c| c.name.clone()),
            collection_name: metadata.as_ref()
                .and_then(|m| m.collection.as_ref())
                .map(|c| c.name.clone()),
            is_token_2022: nft_info.is_token_2022,
            metadata_fetched_at: Some(Utc::now()),
        });
    }

    Ok(WalletSnapshot {
        id: None,
        wallet_address,
        snapshot_time,
        sol_balance,
        sol_balance_lamports,
        total_tokens_count: token_balances.len() as u32,
        total_nfts_count: nft_balances.len() as u32,
        token_balances,
        nft_balances,
    })
}
```

---

### Component 5: Dashboard API Extension

**New Endpoint:** `GET /api/wallet/nfts`

**Response Structure:**

```rust
#[derive(Debug, Serialize)]
pub struct WalletNftsResponse {
    pub nfts: Vec<NftDisplayInfo>,
    pub total_count: u32,
    pub last_updated: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NftDisplayInfo {
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub image_url: Option<String>,
    pub collection: Option<String>,
    pub attributes: Vec<NftAttribute>,
    pub metadata_uri: String,
    pub is_token_2022: bool,
}
```

**Handler:**

```rust
async fn get_wallet_nfts() -> Json<WalletNftsResponse> {
    match get_current_wallet_nfts().await {
        Ok(nfts) => Json(WalletNftsResponse {
            nfts: nfts.into_iter().map(|nft| NftDisplayInfo {
                mint: nft.mint,
                name: nft.name.unwrap_or_else(|| "Unknown NFT".to_string()),
                symbol: nft.symbol.unwrap_or_default(),
                image_url: nft.image_url,
                collection: nft.collection_name,
                attributes: vec![], // Fetch from metadata if needed
                metadata_uri: nft.metadata_uri.unwrap_or_default(),
                is_token_2022: nft.is_token_2022,
            }).collect(),
            total_count: nfts.len() as u32,
            last_updated: Some(Utc::now().to_rfc3339()),
        }),
        Err(_) => Json(WalletNftsResponse {
            nfts: vec![],
            total_count: 0,
            last_updated: None,
        }),
    }
}
```

---

### Component 6: Frontend NFT Tab

**Location:** `src/webserver/templates/scripts/pages/wallet.js`

**Add to SUB_TABS:**

```javascript
const SUB_TABS = [
  { id: "overview", label: '<i class="icon-chart-bar"></i> Overview' },
  { id: "flows", label: '<i class="icon-arrow-right-left"></i> Flows' },
  { id: "holdings", label: '<i class="icon-coins"></i> Tokens' }, // Renamed
  { id: "nfts", label: '<i class="icon-image"></i> NFTs' }, // NEW
  { id: "history", label: '<i class="icon-history"></i> History' },
];
```

**New Render Function:**

```javascript
function renderNfts(container, data) {
  if (!data || !data.nfts || data.nfts.length === 0) {
    container.innerHTML = '<div class="empty-state">No NFTs in wallet</div>';
    return;
  }

  container.innerHTML = `
        <div class="wallet-nfts">
            <div class="nfts-grid" id="nftsGrid"></div>
        </div>
    `;

  renderNftsGrid(data.nfts);
}

function renderNftsGrid(nfts) {
  const grid = document.querySelector("#nftsGrid");
  if (!grid) return;

  grid.innerHTML = nfts
    .map(
      (nft) => `
        <div class="nft-card" data-mint="${escapeHtml(nft.mint)}">
            <div class="nft-image">
                ${
                  nft.image_url
                    ? `<img src="${escapeHtml(nft.image_url)}" alt="${escapeHtml(nft.name)}" loading="lazy"/>`
                    : '<div class="nft-placeholder"><i class="icon-image"></i></div>'
                }
            </div>
            <div class="nft-info">
                <div class="nft-name">${escapeHtml(nft.name)}</div>
                ${
                  nft.collection
                    ? `<div class="nft-collection">${escapeHtml(nft.collection)}</div>`
                    : ""
                }
                <div class="nft-mint">${escapeHtml(nft.mint.substring(0, 8))}...</div>
            </div>
            <div class="nft-actions">
                <button class="btn btn-sm" data-action="view" data-mint="${escapeHtml(nft.mint)}">
                    View
                </button>
                <button class="btn btn-sm" data-action="send" data-mint="${escapeHtml(nft.mint)}">
                    Send
                </button>
            </div>
        </div>
    `
    )
    .join("");
}
```

**CSS:**

```css
.nfts-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 16px;
  padding: 16px 0;
}

.nft-card {
  background: var(--bg-card);
  border: 1px solid var(--border-color);
  border-radius: 12px;
  overflow: hidden;
  transition:
    transform 0.2s,
    box-shadow 0.2s;
  cursor: pointer;
}

.nft-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 16px rgba(0, 0, 0, 0.2);
}

.nft-image {
  width: 100%;
  aspect-ratio: 1;
  background: var(--bg-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
}

.nft-image img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.nft-placeholder {
  font-size: 3rem;
  color: var(--text-muted);
}

.nft-info {
  padding: 12px;
}

.nft-name {
  font-weight: 600;
  font-size: 0.95rem;
  margin-bottom: 4px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.nft-collection {
  font-size: 0.8rem;
  color: var(--text-secondary);
  margin-bottom: 4px;
}

.nft-mint {
  font-size: 0.75rem;
  color: var(--text-muted);
  font-family: monospace;
}

.nft-actions {
  display: flex;
  gap: 8px;
  padding: 0 12px 12px;
}

.nft-actions .btn {
  flex: 1;
}
```

---

## 7. Implementation Priority

### Phase 1: Detection & Separation (Critical)

**Files to Modify:**

1. `src/rpc.rs`
   - Make `extract_nft_mint_if_valid()` public
   - Create `get_all_token_accounts_v2()` with NFT separation
   - Add `NftAccountInfo` struct

2. `src/wallet.rs`
   - Add `nft_balances` table schema
   - Modify `WalletSnapshot` struct
   - Update `collect_wallet_snapshot()` to use v2 RPC method
   - Add NFT balance saving logic

**Expected Outcome:**

- NFTs no longer appear in token holdings
- Separate storage for NFTs
- Foundation for metadata fetching

---

### Phase 2: Metadata Fetching (High Priority)

**New Module:**

1. `src/nfts/` directory
   - `mod.rs` - Module exports
   - `metadata.rs` - Metaplex fetcher
   - `types.rs` - NFT structures
   - `cache.rs` - Metadata caching

**Dependencies to Add:**

```toml
[dependencies]
mpl-token-metadata = "2.0"  # Official Metaplex library
```

**Expected Outcome:**

- Fetch NFT names, symbols, images
- Store in database for quick access
- Background refresh for updated metadata

---

### Phase 3: Dashboard Integration (Medium Priority)

**Files to Modify:**

1. `src/webserver/routes/wallet.rs`
   - Add `get_wallet_nfts()` endpoint
   - Create NFT response structures

2. `src/webserver/templates/scripts/pages/wallet.js`
   - Add "NFTs" tab
   - Implement grid rendering
   - Add view/send actions

3. `src/webserver/templates/styles/pages/wallet.css`
   - Add NFT grid styles
   - Card hover effects
   - Responsive layout

**Expected Outcome:**

- Beautiful NFT gallery in dashboard
- Easy viewing/management of NFTs
- Separate from token holdings

---

### Phase 4: Advanced Features (Optional)

1. **NFT Collections Grouping**
   - Group by collection_mint
   - Show collection stats

2. **NFT Marketplace Links**
   - Magic Eden integration
   - Tensor links
   - Floor price display

3. **NFT Attributes Display**
   - Show traits in detail view
   - Rarity scores
   - Trait filtering

4. **Bulk NFT Actions**
   - Select multiple NFTs
   - Batch send
   - Collection management

---

## 8. Technical Considerations

### Performance

- **Metadata Fetching:** Can be slow (Metaplex account + JSON URI)
  - **Solution:** Async fetch, cache in DB, background refresh
  - **Limit:** Fetch 10 NFTs concurrently max

- **Image Loading:** Large images can slow UI
  - **Solution:** Use `loading="lazy"` on images
  - **Solution:** Show placeholder while loading

- **Database Size:** NFT metadata adds storage
  - **Current:** ~1KB per NFT
  - **Solution:** Cleanup old snapshots regularly

### Rate Limiting

- **RPC Calls:** Each NFT needs 1 getAccountInfo call
  - **Impact:** 20 NFTs = 20 RPC calls
  - **Solution:** Use existing rate limiter
  - **Solution:** Batch fetch when possible

- **HTTP Requests:** Off-chain JSON from various hosts
  - **Impact:** Can be slow/unreliable (Arweave, IPFS)
  - **Solution:** 10-second timeout per request
  - **Solution:** Cache metadata indefinitely

### Error Handling

- **Missing Metadata:** Some NFTs might not have Metaplex accounts
  - **Solution:** Fall back to mint address as name
  - **Solution:** Show generic image placeholder

- **Invalid JSON:** Malformed off-chain metadata
  - **Solution:** Catch parse errors, use on-chain data only
  - **Solution:** Log error for debugging

- **Network Failures:** IPFS/Arweave might be down
  - **Solution:** Retry 3 times with exponential backoff
  - **Solution:** Cache successful fetches permanently

---

## 9. Alternative Approaches Considered

### Option A: Filter NFTs Completely

**Pros:**

- Simplest implementation
- No new code needed
- Lower storage requirements

**Cons:**

- Users can't see their NFTs
- Incomplete wallet view
- Doesn't match user expectations

**Verdict:** ❌ Not user-friendly

---

### Option B: Use Third-Party NFT APIs

**Services:** Helius, SimpleHash, Moralis

**Pros:**

- Pre-aggregated metadata
- Fast responses
- No RPC calls needed

**Cons:**

- External dependency
- API costs
- Rate limits
- May not have all NFTs

**Verdict:** ⚠️ Consider for Phase 4

---

### Option C: Treat NFTs as Tokens (Current)

**Pros:**

- No code changes
- Works with existing system

**Cons:**

- Confusing UI (NFTs mixed with tokens)
- No metadata/images
- Wrong balance display
- No collection grouping

**Verdict:** ❌ Current problem

---

### Option D: Hybrid Approach (Recommended)

**Implementation:**

1. Detect NFTs at RPC level (Phase 1)
2. Store separately in database (Phase 1)
3. Fetch Metaplex metadata (Phase 2)
4. Display in dedicated tab (Phase 3)
5. Add advanced features as needed (Phase 4)

**Pros:**

- Systematic solution
- Incremental implementation
- No breaking changes
- Extensible architecture

**Cons:**

- Moderate complexity
- Requires new module
- More database tables

**Verdict:** ✅ RECOMMENDED

---

## 10. Migration Strategy

### Backward Compatibility

**Existing Data:**

- Current wallet.db has NFTs in `token_balances`
- Must not break existing snapshots

**Migration SQL:**

```sql
-- Step 1: Create new NFT table
CREATE TABLE IF NOT EXISTS nft_balances (...);

-- Step 2: Migrate existing NFTs (decimals = 0 OR NULL, balance = 1)
INSERT INTO nft_balances (snapshot_id, mint, account_address, is_token_2022, created_at)
SELECT
    snapshot_id,
    mint,
    '', -- account_address unknown from old data
    is_token_2022,
    created_at
FROM token_balances
WHERE balance = 1 AND (decimals IS NULL OR decimals = 0);

-- Step 3: Optional - Remove NFTs from token_balances
-- DELETE FROM token_balances WHERE balance = 1 AND (decimals IS NULL OR decimals = 0);
-- Note: Keep for now to avoid data loss
```

### Rollback Plan

If implementation fails:

1. Keep `get_all_token_accounts()` (old function)
2. Don't use `get_all_token_accounts_v2()`
3. NFTs continue showing as tokens (current behavior)
4. No data loss

---

## 11. Testing Plan

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_nft_mint_valid() {
        let json = serde_json::json!({
            "account": {
                "data": {
                    "parsed": {
                        "info": {
                            "mint": "CmpPkVrJrvZeZZTz3W3u8pbrQbmzuiCbAxFcWKo8Ux1G",
                            "tokenAmount": {
                                "amount": "1",
                                "decimals": 0,
                                "uiAmount": 1.0
                            }
                        }
                    }
                }
            }
        });

        let result = extract_nft_mint_if_valid(&json);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), "CmpPkVrJrvZeZZTz3W3u8pbrQbmzuiCbAxFcWKo8Ux1G");
    }

    #[test]
    fn test_extract_nft_mint_fungible_token() {
        let json = serde_json::json!({
            "account": {
                "data": {
                    "parsed": {
                        "info": {
                            "mint": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
                            "tokenAmount": {
                                "amount": "1000000",
                                "decimals": 6,
                                "uiAmount": 1.0
                            }
                        }
                    }
                }
            }
        });

        let result = extract_nft_mint_if_valid(&json);
        assert!(result.is_none()); // decimals = 6, not NFT
    }
}
```

### Integration Tests

1. **Wallet Collection:**
   - Create test wallet with 1 NFT + 1 token
   - Call `get_all_token_accounts_v2()`
   - Verify separation

2. **Metadata Fetching:**
   - Use known NFT mint
   - Fetch Metaplex metadata
   - Verify image URL returned

3. **Database Storage:**
   - Save snapshot with NFTs
   - Query `nft_balances` table
   - Verify correct storage

### Manual Testing Checklist

- [ ] Wallet with 0 NFTs → Holdings tab shows tokens only
- [ ] Wallet with 1+ NFTs → NFTs tab appears, shows grid
- [ ] NFT images load correctly
- [ ] NFT names display (not truncated mints)
- [ ] Token holdings don't include NFTs
- [ ] Balance calculations correct (no NFTs affecting totals)
- [ ] Dashboard refresh updates NFT metadata
- [ ] Old snapshots still viewable

---

## 12. Documentation Requirements

### Code Documentation

- [ ] Add module-level docs to `src/nfts/mod.rs`
- [ ] Document NFT detection criteria
- [ ] Comment Metaplex PDA derivation
- [ ] Explain metadata caching strategy

### User Documentation

- [ ] Update README with NFT features
- [ ] Add screenshots of NFT tab
- [ ] Explain NFT vs Token difference
- [ ] Document supported NFT standards

### API Documentation

- [ ] Document `/api/wallet/nfts` endpoint
- [ ] Add NFT response examples
- [ ] Update Swagger/OpenAPI spec

---

## 13. Known Limitations

### Current Implementation Gaps

1. **No Token-2022 NFT Extensions**
   - Token-2022 can have different NFT implementations
   - Current detection might miss some variants

2. **No Compressed NFTs Support**
   - Metaplex compressed NFTs use different structure
   - Require specialized fetching logic

3. **No pNFT Support**
   - Programmable NFTs have additional rules
   - Need separate handling

### Future Enhancements

1. **Collection Floor Price**
   - Integrate Magic Eden API
   - Show estimated value

2. **NFT Verification**
   - Check if collection is verified
   - Show verification badge

3. **Bulk Actions**
   - Send multiple NFTs
   - List on marketplaces

---

## 14. Security Considerations

### Metadata Fetching

- **Malicious URIs:** Off-chain metadata could point to malicious sites
  - **Mitigation:** Validate URL schemes (https only)
  - **Mitigation:** Timeout requests (10s max)
  - **Mitigation:** Sanitize HTML in NFT descriptions

- **Large Images:** Could cause memory issues
  - **Mitigation:** Proxy images through backend
  - **Mitigation:** Size limits on image downloads
  - **Mitigation:** Lazy loading in UI

### Transaction Safety

- **NFT Sends:** Must verify recipient
  - **Mitigation:** Confirm dialog with full address
  - **Mitigation:** Check transaction before signing

---

## 15. Monitoring & Metrics

### Key Metrics to Track

1. **NFT Detection Rate**
   - How many NFTs detected vs total accounts
   - False positive rate

2. **Metadata Fetch Success**
   - Percentage of NFTs with complete metadata
   - Average fetch time

3. **UI Performance**
   - NFT tab load time
   - Image load time
   - Grid rendering performance

### Logging Strategy

```rust
logger::info(
    LogTag::Wallet,
    &format!(
        "NFT collection: detected {} NFTs, {} tokens, fetch_time={}ms",
        nft_count, token_count, fetch_duration
    )
);

logger::debug(
    LogTag::Wallet,
    &format!(
        "NFT metadata fetched: mint={} name={} image={}",
        mint, metadata.name, metadata.image_url.is_some()
    )
);
```

---

## 16. Conclusion

### Summary of Findings

1. **NFT detection function exists but is unused**
2. **NFTs currently mixed with tokens in database/UI**
3. **No metadata fetching for NFTs (different from tokens)**
4. **Systematic solution requires multi-phase implementation**

### Recommended Action

**Implement Hybrid Approach (Option D) in 4 phases:**

1. Detection & Separation (1-2 days)
2. Metadata Fetching (2-3 days)
3. Dashboard Integration (2-3 days)
4. Advanced Features (ongoing)

### Expected Benefits

- ✅ Clean separation of NFTs and tokens
- ✅ Beautiful NFT gallery in dashboard
- ✅ Correct balance calculations
- ✅ Complete wallet view for users
- ✅ Foundation for NFT-specific features

### Risk Assessment

**Low Risk:**

- Changes are additive (no breaking changes)
- Existing data preserved
- Backward compatible
- Can rollback easily

---

## Appendix A: File Modification Checklist

### Phase 1 Files

- [ ] `src/rpc.rs` - Add NFT detection to `get_all_token_accounts()`
- [ ] `src/wallet.rs` - Add `nft_balances` table and schema
- [ ] `src/wallet.rs` - Modify `WalletSnapshot` struct
- [ ] `src/wallet.rs` - Update `collect_wallet_snapshot()` function

### Phase 2 Files

- [ ] `src/nfts/mod.rs` - New module
- [ ] `src/nfts/metadata.rs` - Metaplex fetcher
- [ ] `src/nfts/types.rs` - NFT structures
- [ ] `src/nfts/cache.rs` - Metadata caching
- [ ] `Cargo.toml` - Add `mpl-token-metadata` dependency

### Phase 3 Files

- [ ] `src/webserver/routes/wallet.rs` - Add NFT endpoints
- [ ] `src/webserver/templates/scripts/pages/wallet.js` - Add NFT tab
- [ ] `src/webserver/templates/styles/pages/wallet.css` - Add NFT styles
- [ ] `src/webserver/templates/pages/wallet.html` - Update structure

---

## Appendix B: Example NFT Detection

### Test Case: CmpPkVrJrvZeZZTz3W3u8pbrQbmzuiCbAxFcWKo8Ux1G

**Raw RPC Response:**

```json
{
  "account": {
    "data": {
      "parsed": {
        "info": {
          "mint": "CmpPkVrJrvZeZZTz3W3u8pbrQbmzuiCbAxFcWKo8Ux1G",
          "tokenAmount": {
            "amount": "1",
            "decimals": 0,
            "uiAmount": 1.0,
            "uiAmountString": "1"
          },
          "owner": "YourWalletAddressHere",
          "state": "initialized"
        }
      }
    },
    "owner": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
  }
}
```

**Detection Logic:**

```rust
// decimals == 0 ✓
// uiAmount == 1.0 ✓
// RESULT: IS NFT ✓
```

**Expected Behavior:**

- ❌ Before: Shows in "Token Holdings" as "CmpP..." with no metadata
- ✅ After: Shows in "NFTs" tab with image, name, collection

---

**END OF DOCUMENT**
