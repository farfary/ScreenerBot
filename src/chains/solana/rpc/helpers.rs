//! Small Solana-typed helpers used by RPC stats/debug surfaces.

/// Parse pubkey helper (delegate to `chains::solana::accounts`)
pub fn parse_pubkey(
    address: &str,
) -> Result<crate::chains::solana::solana_sdk::pubkey::Pubkey, String> {
    crate::chains::solana::accounts::parse_pubkey_safe(address)
}

/// Return the SPL Token program id (use constant)
pub fn spl_token_program_id() -> &'static str {
    crate::chains::solana::constants::SPL_TOKEN_PROGRAM_ID
}
