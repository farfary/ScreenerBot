use rusqlite::{Connection, Result as SqliteResult, Row};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityInfo {
    pub mint: String,
    pub token_program: Option<String>,
    pub creator: Option<String>,
    pub creator_balance: u64,

    // Token metadata
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: Option<u8>,
    pub supply: Option<u64>,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,

    // Security scoring
    pub score: i32,
    pub score_normalised: i32,
    pub rugged: bool,
    pub risks: Vec<SecurityRisk>,

    // Market information
    pub markets: Vec<MarketInfo>,
    pub total_market_liquidity: f64,
    pub total_stable_liquidity: f64,
    pub total_lp_providers: i32,
    pub total_holders: i32,
    pub price: f64,

    // Holder analysis
    pub top_holders: Vec<HolderInfo>,
    pub graph_insiders_detected: i32,

    // Analysis timestamps
    pub detected_at: String,
    pub analyzed_at: String,

    // Raw response for future extensibility
    pub raw_response: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRisk {
    pub name: String,
    pub value: String,
    pub description: String,
    pub score: i32,
    pub level: String, // "warn", "danger", etc.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketInfo {
    pub pubkey: String,
    pub market_type: String,
    pub mint_a: String,
    pub mint_b: String,
    pub mint_lp: Option<String>,
    pub liquidity_a: String,
    pub liquidity_b: String,
    pub lp_locked_pct: f64,
    pub lp_unlocked: f64,
    pub base_price: f64,
    pub quote_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HolderInfo {
    pub address: String,
    pub amount: u64,
    pub pct: f64,
    pub owner: String,
    pub insider: bool,
}

pub struct SecurityDatabase {
    conn: Connection,
}

impl SecurityDatabase {
    pub fn new(db_path: &str) -> SqliteResult<Self> {
        let conn = Connection::open(db_path)?;
        // Configure pragmas for better concurrency/perf
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
        let _ = conn.pragma_update(None, "synchronous", "NORMAL");
        let _ = conn.pragma_update(None, "cache_size", 10000);
        let _ = conn.pragma_update(None, "temp_store", "memory");
        let _ = conn.busy_timeout(std::time::Duration::from_millis(30_000));
        let db = SecurityDatabase { conn };
        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> SqliteResult<()> {
        // Main security info table with full data
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS security_info (
                mint TEXT PRIMARY KEY,
                token_program TEXT,
                creator TEXT,
                creator_balance INTEGER NOT NULL DEFAULT 0,
                
                -- Token metadata
                symbol TEXT,
                name TEXT,
                decimals INTEGER,
                supply INTEGER,
                mint_authority TEXT,
                freeze_authority TEXT,
                
                -- Security scoring
                score INTEGER NOT NULL DEFAULT 0,
                score_normalised INTEGER NOT NULL DEFAULT 0,
                rugged INTEGER NOT NULL DEFAULT 0,
                
                -- Market information
                total_market_liquidity REAL NOT NULL DEFAULT 0.0,
                total_stable_liquidity REAL NOT NULL DEFAULT 0.0,
                total_lp_providers INTEGER NOT NULL DEFAULT 0,
                total_holders INTEGER NOT NULL DEFAULT 0,
                price REAL NOT NULL DEFAULT 0.0,
                
                -- Holder analysis
                graph_insiders_detected INTEGER NOT NULL DEFAULT 0,
                
                -- Timestamps
                detected_at TEXT NOT NULL,
                analyzed_at TEXT NOT NULL DEFAULT (datetime('now')),
                
                -- Full raw response for extensibility
                raw_response TEXT NOT NULL
            )
            "#,
            [],
        )?;

        // Security risks table (normalized)
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS security_risks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mint TEXT NOT NULL,
                name TEXT NOT NULL,
                value TEXT,
                description TEXT NOT NULL,
                score INTEGER NOT NULL,
                level TEXT NOT NULL,
                FOREIGN KEY (mint) REFERENCES security_info(mint)
            )
            "#,
            [],
        )?;

        // Markets table (normalized)
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS security_markets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mint TEXT NOT NULL,
                pubkey TEXT NOT NULL,
                market_type TEXT NOT NULL,
                mint_a TEXT NOT NULL,
                mint_b TEXT NOT NULL,
                mint_lp TEXT,
                liquidity_a TEXT NOT NULL,
                liquidity_b TEXT NOT NULL,
                lp_locked_pct REAL NOT NULL DEFAULT 0.0,
                lp_unlocked REAL NOT NULL DEFAULT 0.0,
                base_price REAL NOT NULL DEFAULT 0.0,
                quote_price REAL NOT NULL DEFAULT 0.0,
                FOREIGN KEY (mint) REFERENCES security_info(mint)
            )
            "#,
            [],
        )?;

        // Top holders table (normalized)
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS security_holders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                mint TEXT NOT NULL,
                address TEXT NOT NULL,
                amount INTEGER NOT NULL,
                pct REAL NOT NULL,
                owner TEXT NOT NULL,
                insider INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (mint) REFERENCES security_info(mint)
            )
            "#,
            [],
        )?;

        // Create indices for performance
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_security_info_analyzed ON security_info(analyzed_at)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_security_info_score ON security_info(score_normalised)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_security_risks_mint ON security_risks(mint)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_security_risks_level ON security_risks(level)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_security_markets_mint ON security_markets(mint)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_security_markets_type ON security_markets(market_type)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_security_holders_mint ON security_holders(mint)",
            [],
        )?;

        Ok(())
    }

    pub fn store_security_info(&self, info: &SecurityInfo) -> SqliteResult<()> {
        let tx = self.conn.unchecked_transaction()?;

        // Insert main security info
        tx.execute(
            r#"
            INSERT OR REPLACE INTO security_info 
            (mint, token_program, creator, creator_balance, symbol, name, decimals, supply,
             mint_authority, freeze_authority, score, score_normalised, rugged,
             total_market_liquidity, total_stable_liquidity, total_lp_providers, total_holders,
             price, graph_insiders_detected, detected_at, raw_response)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)
            "#,
            rusqlite::params![
                info.mint,
                info.token_program,
                info.creator,
                info.creator_balance,
                info.symbol,
                info.name,
                info.decimals,
                info.supply,
                info.mint_authority,
                info.freeze_authority,
                info.score,
                info.score_normalised,
                info.rugged as i32,
                info.total_market_liquidity,
                info.total_stable_liquidity,
                info.total_lp_providers,
                info.total_holders,
                info.price,
                info.graph_insiders_detected,
                info.detected_at,
                info.raw_response
            ]
        )?;

        // Clear and insert risks
        tx.execute("DELETE FROM security_risks WHERE mint = ?1", [&info.mint])?;
        for risk in &info.risks {
            tx.execute(
                "INSERT INTO security_risks (mint, name, value, description, score, level) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    info.mint,
                    risk.name,
                    risk.value,
                    risk.description,
                    risk.score,
                    risk.level
                ]
            )?;
        }

        // Clear and insert markets
        tx.execute("DELETE FROM security_markets WHERE mint = ?1", [&info.mint])?;
        for market in &info.markets {
            tx.execute(
                r#"
                INSERT INTO security_markets 
                (mint, pubkey, market_type, mint_a, mint_b, mint_lp, liquidity_a, liquidity_b,
                 lp_locked_pct, lp_unlocked, base_price, quote_price)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
                "#,
                rusqlite::params![
                    info.mint,
                    market.pubkey,
                    market.market_type,
                    market.mint_a,
                    market.mint_b,
                    market.mint_lp,
                    market.liquidity_a,
                    market.liquidity_b,
                    market.lp_locked_pct,
                    market.lp_unlocked,
                    market.base_price,
                    market.quote_price
                ],
            )?;
        }

        // Clear and insert top holders
        tx.execute("DELETE FROM security_holders WHERE mint = ?1", [&info.mint])?;
        for holder in &info.top_holders {
            tx.execute(
                "INSERT INTO security_holders (mint, address, amount, pct, owner, insider) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    info.mint,
                    holder.address,
                    holder.amount,
                    holder.pct,
                    holder.owner,
                    holder.insider as i32
                ]
            )?;
        }

        tx.commit()?;
        Ok(())
    }

    pub fn get_security_info(&self, mint: &str) -> SqliteResult<Option<SecurityInfo>> {
        let mut stmt = self
            .conn
            .prepare("SELECT * FROM security_info WHERE mint = ?1")?;

        let mut rows = stmt.query_map([mint], |row| {
            Ok(SecurityInfo {
                mint: row.get("mint")?,
                token_program: row.get("token_program")?,
                creator: row.get("creator")?,
                creator_balance: row.get("creator_balance")?,
                symbol: row.get("symbol")?,
                name: row.get("name")?,
                decimals: row.get("decimals")?,
                supply: row.get("supply")?,
                mint_authority: row.get("mint_authority")?,
                freeze_authority: row.get("freeze_authority")?,
                score: row.get("score")?,
                score_normalised: row.get("score_normalised")?,
                rugged: row.get::<_, i32>("rugged")? != 0,
                total_market_liquidity: row.get("total_market_liquidity")?,
                total_stable_liquidity: row.get("total_stable_liquidity")?,
                total_lp_providers: row.get("total_lp_providers")?,
                total_holders: row.get("total_holders")?,
                price: row.get("price")?,
                graph_insiders_detected: row.get("graph_insiders_detected")?,
                detected_at: row.get("detected_at")?,
                analyzed_at: row.get("analyzed_at")?,
                raw_response: row.get("raw_response")?,
                risks: Vec::new(),       // Will be filled below
                markets: Vec::new(),     // Will be filled below
                top_holders: Vec::new(), // Will be filled below
            })
        })?;

        if let Some(mut info) = rows.next().transpose()? {
            // Load risks
            let mut risk_stmt = self
                .conn
                .prepare("SELECT * FROM security_risks WHERE mint = ?1")?;
            let risk_rows = risk_stmt.query_map([mint], |row| {
                Ok(SecurityRisk {
                    name: row.get("name")?,
                    value: row.get("value")?,
                    description: row.get("description")?,
                    score: row.get("score")?,
                    level: row.get("level")?,
                })
            })?;
            for risk in risk_rows {
                info.risks.push(risk?);
            }

            // Load markets
            let mut market_stmt = self
                .conn
                .prepare("SELECT * FROM security_markets WHERE mint = ?1")?;
            let market_rows = market_stmt.query_map([mint], |row| {
                Ok(MarketInfo {
                    pubkey: row.get("pubkey")?,
                    market_type: row.get("market_type")?,
                    mint_a: row.get("mint_a")?,
                    mint_b: row.get("mint_b")?,
                    mint_lp: row.get("mint_lp")?,
                    liquidity_a: row.get("liquidity_a")?,
                    liquidity_b: row.get("liquidity_b")?,
                    lp_locked_pct: row.get("lp_locked_pct")?,
                    lp_unlocked: row.get("lp_unlocked")?,
                    base_price: row.get("base_price")?,
                    quote_price: row.get("quote_price")?,
                })
            })?;
            for market in market_rows {
                info.markets.push(market?);
            }

            // Load top holders
            let mut holder_stmt = self
                .conn
                .prepare("SELECT * FROM security_holders WHERE mint = ?1 ORDER BY pct DESC")?;
            let holder_rows = holder_stmt.query_map([mint], |row| {
                Ok(HolderInfo {
                    address: row.get("address")?,
                    amount: row.get("amount")?,
                    pct: row.get("pct")?,
                    owner: row.get("owner")?,
                    insider: row.get::<_, i32>("insider")? != 0,
                })
            })?;
            for holder in holder_rows {
                info.top_holders.push(holder?);
            }

            Ok(Some(info))
        } else {
            Ok(None)
        }
    }

    pub fn is_stale(&self, mint: &str, max_age_hours: i64) -> SqliteResult<bool> {
        let mut stmt = self.conn.prepare(
            "SELECT datetime(analyzed_at, '+' || ?2 || ' hours') < datetime('now') FROM security_info WHERE mint = ?1"
        )?;

        let mut rows = stmt.query_map(rusqlite::params![mint, max_age_hours], |row| {
            Ok(row.get::<_, bool>(0)?)
        })?;

        Ok(rows.next().transpose()?.unwrap_or(true))
    }

    pub fn get_security_stats(&self) -> SqliteResult<HashMap<String, i64>> {
        let mut stats = HashMap::new();

        let mut stmt = self.conn.prepare("SELECT COUNT(*) FROM security_info")?;
        stats.insert("total".to_string(), stmt.query_row([], |row| row.get(0))?);

        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM security_info WHERE rugged = 0")?;
        stats.insert("safe".to_string(), stmt.query_row([], |row| row.get(0))?);

        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM security_info WHERE score_normalised >= 70")?;
        stats.insert(
            "high_score".to_string(),
            stmt.query_row([], |row| row.get(0))?,
        );

        Ok(stats)
    }

    pub fn count_safe_tokens(&self) -> SqliteResult<i64> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM security_info WHERE score_normalised >= 70")?;
        stmt.query_row([], |row| row.get(0))
    }

    pub fn count_warning_tokens(&self) -> SqliteResult<i64> {
        let mut stmt = self.conn.prepare(
            "SELECT COUNT(*) FROM security_info WHERE score_normalised >= 40 AND score_normalised < 70"
        )?;
        stmt.query_row([], |row| row.get(0))
    }

    pub fn count_danger_tokens(&self) -> SqliteResult<i64> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM security_info WHERE score_normalised < 40")?;
        stmt.query_row([], |row| row.get(0))
    }

    pub fn count_pump_fun_tokens(&self) -> SqliteResult<i64> {
        let mut stmt = self
            .conn
            .prepare("SELECT COUNT(*) FROM security_info WHERE raw_response LIKE '%pump.fun%'")?;
        stmt.query_row([], |row| row.get(0))
    }

    /// Count tokens present in tokens.db that do not have a corresponding row in security_info
    /// Uses separate connection to avoid ATTACH race conditions.
    pub fn count_tokens_without_security(&self) -> SqliteResult<i64> {
        // Use separate connection to tokens.db to avoid ATTACH race conditions
        let tokens_conn = Connection::open("data/tokens.db")?;

        // Get all token mints from tokens.db
        let mut tokens_stmt = tokens_conn.prepare("SELECT mint FROM tokens")?;
        let token_rows = tokens_stmt.query_map([], |row| {
            let mint: String = row.get(0)?;
            Ok(mint)
        })?;

        let mut tokens_without_security = 0i64;

        // Check each token mint against security_info table
        let mut security_stmt = self
            .conn
            .prepare("SELECT 1 FROM security_info WHERE mint = ?1 LIMIT 1")?;

        for token_result in token_rows {
            let mint = token_result?;
            let has_security = security_stmt.query_row([&mint], |_| Ok(())).is_ok();
            if !has_security {
                tokens_without_security += 1;
            }
        }

        Ok(tokens_without_security)
    }

    /// Get list of token mints that don't have security info
    /// Used by background monitoring task to fetch security data for unprocessed tokens
    /// Uses separate connection to avoid ATTACH race conditions.
    pub fn get_tokens_without_security(&self) -> SqliteResult<Vec<String>> {
        // Use separate connection to tokens.db to avoid ATTACH race conditions
        let tokens_conn = Connection::open("data/tokens.db")?;

        // Get token mints from tokens.db ordered by liquidity (highest first) with no hard limit
        // Prioritize high-liquidity tokens to ensure security info is populated for tradable assets first
        let mut tokens_stmt =
            tokens_conn.prepare("SELECT mint FROM tokens ORDER BY liquidity_usd DESC")?;
        let token_rows = tokens_stmt.query_map([], |row| {
            let mint: String = row.get(0)?;
            Ok(mint)
        })?;

        let mut tokens_without_security = Vec::new();

        // Check each token mint against security_info table
        let mut security_stmt = self
            .conn
            .prepare("SELECT 1 FROM security_info WHERE mint = ?1 LIMIT 1")?;

        for token_result in token_rows {
            let mint = token_result?;
            let has_security = security_stmt.query_row([&mint], |_| Ok(())).is_ok();
            if !has_security {
                tokens_without_security.push(mint);
            }
        }

        Ok(tokens_without_security)
    }

    // =============================================================================
    // ASYNC-SAFE WRAPPERS FOR WEBSERVER ROUTES
    // =============================================================================

    /// Async-safe method to get security info by mint
    ///
    /// Use this in async contexts (webserver routes) instead of creating SecurityDatabase
    /// and calling get_security_info() directly, which would block the async runtime.
    pub async fn get_security_info_async(mint: &str) -> Result<Option<SecurityInfo>, String> {
        let mint = mint.to_string();
        tokio::task::spawn_blocking(move || {
            let db = SecurityDatabase::new("data/security.db")
                .map_err(|e| format!("Failed to create security database: {}", e))?;
            db.get_security_info(&mint)
                .map_err(|e| format!("Failed to query security database: {}", e))
        })
        .await
        .map_err(|e| format!("Task join error: {}", e))?
    }
}

// Helper function to parse Rugcheck JSON response into SecurityInfo
pub fn parse_rugcheck_response(raw_json: &str) -> Result<SecurityInfo, serde_json::Error> {
    let response: Value = serde_json::from_str(raw_json)?;

    // Extract main fields
    let mint = response["mint"].as_str().unwrap_or("").to_string();
    let token_program = response["tokenProgram"].as_str().map(|s| s.to_string());
    let creator = response["creator"].as_str().map(|s| s.to_string());
    let creator_balance = response["creatorBalance"].as_u64().unwrap_or(0);

    // Extract token metadata
    let token_meta = &response["tokenMeta"];
    let symbol = token_meta["symbol"].as_str().map(|s| s.to_string());
    let name = token_meta["name"].as_str().map(|s| s.to_string());

    let token_info = &response["token"];
    let decimals = token_info["decimals"].as_u64().map(|d| d as u8);
    let supply = token_info["supply"].as_u64();
    let mint_authority = token_info["mintAuthority"].as_str().map(|s| s.to_string());
    let freeze_authority = token_info["freezeAuthority"]
        .as_str()
        .map(|s| s.to_string());

    // Extract security info
    let score = response["score"].as_i64().unwrap_or(0) as i32;
    let score_normalised = response["score_normalised"].as_i64().unwrap_or(0) as i32;
    let rugged = response["rugged"].as_bool().unwrap_or(false);

    // Extract market info
    let total_market_liquidity = response["totalMarketLiquidity"].as_f64().unwrap_or(0.0);
    let total_stable_liquidity = response["totalStableLiquidity"].as_f64().unwrap_or(0.0);
    let total_lp_providers = response["totalLPProviders"].as_i64().unwrap_or(0) as i32;
    let total_holders = response["totalHolders"].as_i64().unwrap_or(0) as i32;
    let price = response["price"].as_f64().unwrap_or(0.0);
    let graph_insiders_detected = response["graphInsidersDetected"].as_i64().unwrap_or(0) as i32;

    // Extract detected_at
    let detected_at = response["detectedAt"].as_str().unwrap_or("").to_string();

    // Parse risks array
    let mut risks = Vec::new();
    if let Some(risks_array) = response["risks"].as_array() {
        for risk in risks_array {
            risks.push(SecurityRisk {
                name: risk["name"].as_str().unwrap_or("").to_string(),
                value: risk["value"].as_str().unwrap_or("").to_string(),
                description: risk["description"].as_str().unwrap_or("").to_string(),
                score: risk["score"].as_i64().unwrap_or(0) as i32,
                level: risk["level"].as_str().unwrap_or("").to_string(),
            });
        }
    }

    // Parse markets array
    let mut markets = Vec::new();
    if let Some(markets_array) = response["markets"].as_array() {
        for market in markets_array {
            let lp = &market["lp"];
            markets.push(MarketInfo {
                pubkey: market["pubkey"].as_str().unwrap_or("").to_string(),
                market_type: market["marketType"].as_str().unwrap_or("").to_string(),
                mint_a: market["mintA"].as_str().unwrap_or("").to_string(),
                mint_b: market["mintB"].as_str().unwrap_or("").to_string(),
                mint_lp: market["mintLP"].as_str().map(|s| s.to_string()),
                liquidity_a: market["liquidityA"].as_str().unwrap_or("").to_string(),
                liquidity_b: market["liquidityB"].as_str().unwrap_or("").to_string(),
                lp_locked_pct: lp["lpLockedPct"].as_f64().unwrap_or(0.0),
                lp_unlocked: lp["lpUnlocked"].as_f64().unwrap_or(0.0),
                base_price: lp["basePrice"].as_f64().unwrap_or(0.0),
                quote_price: lp["quotePrice"].as_f64().unwrap_or(0.0),
            });
        }
    }

    // Parse top holders array
    let mut top_holders = Vec::new();
    if let Some(holders_array) = response["topHolders"].as_array() {
        for holder in holders_array {
            top_holders.push(HolderInfo {
                address: holder["address"].as_str().unwrap_or("").to_string(),
                amount: holder["amount"].as_u64().unwrap_or(0),
                pct: holder["pct"].as_f64().unwrap_or(0.0),
                owner: holder["owner"].as_str().unwrap_or("").to_string(),
                insider: holder["insider"].as_bool().unwrap_or(false),
            });
        }
    }

    Ok(SecurityInfo {
        mint,
        token_program,
        creator,
        creator_balance,
        symbol,
        name,
        decimals,
        supply,
        mint_authority,
        freeze_authority,
        score,
        score_normalised,
        rugged,
        risks,
        markets,
        total_market_liquidity,
        total_stable_liquidity,
        total_lp_providers,
        total_holders,
        price,
        top_holders,
        graph_insiders_detected,
        detected_at,
        analyzed_at: chrono::Utc::now().to_rfc3339(),
        raw_response: raw_json.to_string(),
    })
}
