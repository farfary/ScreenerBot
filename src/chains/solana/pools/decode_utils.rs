//! Pubkey-shaped byte parsing for pool account decoders.
//!
//! Solana-owned: reading a 32-byte pubkey out of raw account data (and
//! formatting/round-tripping it through `Pubkey`) is Solana-specific
//! machinery. The chain-neutral, mint/vault-string-shaped helpers this
//! complements (SOL detection, token-pair orientation) live in
//! `crate::pools::utils`.

use crate::chains::solana::solana_sdk::pubkey::Pubkey;

/// Read a pubkey from data at given offset, advancing the offset
pub fn read_pubkey_at_offset(data: &[u8], offset: &mut usize) -> Result<String, String> {
    if *offset + 32 > data.len() {
        return Err(format!(
            "Offset {} + 32 exceeds data length {}",
            *offset,
            data.len()
        ));
    }

    let pubkey_bytes = &data[*offset..*offset + 32];
    *offset += 32;

    let pubkey = Pubkey::new_from_array(
        pubkey_bytes
            .try_into()
            .map_err(|_| "Invalid pubkey bytes".to_owned())?,
    );

    Ok(pubkey.to_string())
}

/// Read a pubkey from data at fixed offset without advancing
pub fn read_pubkey_at(data: &[u8], offset: usize) -> Option<String> {
    if offset + 32 > data.len() {
        return None;
    }
    let pk = Pubkey::new_from_array(data[offset..offset + 32].try_into().ok()?);
    Some(pk.to_string())
}

/// Read a pubkey as Pubkey struct from data at given offset, advancing the offset
pub fn read_pubkey_struct_at_offset(
    data: &[u8],
    offset: &mut usize,
) -> Result<Pubkey, &'static str> {
    if data.len() < *offset + 32 {
        return Err("Insufficient data for pubkey");
    }
    let pubkey_bytes = &data[*offset..*offset + 32];
    *offset += 32;
    Pubkey::try_from(pubkey_bytes).map_err(|_| "Invalid pubkey")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_pubkey_at_offset_advances_and_round_trips() {
        let pk = Pubkey::new_unique();
        let mut data = vec![0u8; 4];
        data.extend_from_slice(&pk.to_bytes());
        let mut offset = 4;
        let read = read_pubkey_at_offset(&data, &mut offset).unwrap();
        assert_eq!(read, pk.to_string());
        assert_eq!(offset, 36);
    }

    #[test]
    fn read_pubkey_at_offset_rejects_short_data() {
        let data = vec![0u8; 10];
        let mut offset = 0;
        assert!(read_pubkey_at_offset(&data, &mut offset).is_err());
    }

    #[test]
    fn read_pubkey_at_does_not_advance() {
        let pk = Pubkey::new_unique();
        let data = pk.to_bytes().to_vec();
        assert_eq!(read_pubkey_at(&data, 0), Some(pk.to_string()));
    }

    #[test]
    fn read_pubkey_struct_at_offset_returns_pubkey_type() {
        let pk = Pubkey::new_unique();
        let data = pk.to_bytes().to_vec();
        let mut offset = 0;
        let read = read_pubkey_struct_at_offset(&data, &mut offset).unwrap();
        assert_eq!(read, pk);
        assert_eq!(offset, 32);
    }
}
