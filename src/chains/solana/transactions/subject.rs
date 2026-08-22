//! Solana conversions for the shared transaction [`Subject`].

use std::str::FromStr;

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use crate::chains::{AccountId, ChainError, ChainId};
use crate::transactions::Subject;

/// Builds a shared subject from a Solana public key.
pub fn from_pubkey(pubkey: Pubkey) -> Subject {
    let account = AccountId::new(ChainId::Solana, pubkey.to_string())
        .expect("a Solana pubkey encoding is never empty");
    Subject::from_account(account)
}

/// Parses a Solana address into a shared subject.
pub fn try_from_address(address: &str) -> Result<Subject, ChainError> {
    Ok(from_pubkey(parse_solana_pubkey(address, ChainId::Solana)?))
}

/// Converts a chain-qualified account into a Solana-backed subject.
pub fn try_from_account(account: AccountId) -> Result<Subject, ChainError> {
    require_solana(account.chain())?;
    let pubkey = parse_solana_pubkey(account.address(), account.chain())?;
    Ok(from_pubkey(pubkey))
}

/// Recovers the Solana public key for a shared subject.
pub fn try_to_pubkey(subject: &Subject) -> Result<Pubkey, ChainError> {
    try_account_to_pubkey(subject.account())
}

/// Recovers the Solana public key for a chain-qualified account.
pub fn try_account_to_pubkey(account: &AccountId) -> Result<Pubkey, ChainError> {
    require_solana(account.chain())?;
    parse_solana_pubkey(account.address(), account.chain())
}

fn require_solana(chain: ChainId) -> Result<(), ChainError> {
    if chain != ChainId::Solana {
        return Err(ChainError::WrongChain {
            expected: ChainId::Solana,
            actual: chain,
        });
    }
    Ok(())
}

fn parse_solana_pubkey(address: &str, chain: ChainId) -> Result<Pubkey, ChainError> {
    Pubkey::from_str(address).map_err(|_| ChainError::InvalidAccount {
        chain,
        value: address.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pubkey_round_trips_through_shared_subject() {
        let pubkey = Pubkey::new_unique();
        let subject = from_pubkey(pubkey);

        assert_eq!(subject.chain(), ChainId::Solana);
        assert_eq!(subject.address(), pubkey.to_string());
        assert_eq!(try_to_pubkey(&subject).unwrap(), pubkey);
        assert_eq!(try_from_address(&pubkey.to_string()).unwrap(), subject);
        assert_eq!(
            try_from_account(subject.account().clone()).unwrap(),
            subject
        );
    }

    #[test]
    fn invalid_or_wrong_chain_identities_are_rejected() {
        assert!(matches!(
            try_from_address("not-a-valid-pubkey"),
            Err(ChainError::InvalidAccount { .. })
        ));
        assert!(matches!(
            "evm".parse::<ChainId>(),
            Err(ChainError::UnsupportedChain { .. })
        ));
        assert!(matches!(
            serde_json::from_str::<AccountId>(r#"{"chain":"evm","address":"0xabc"}"#),
            Err(_)
        ));

        let invalid = AccountId::new(ChainId::Solana, "not-a-valid-pubkey").unwrap();
        assert!(matches!(
            try_from_account(invalid),
            Err(ChainError::InvalidAccount { .. })
        ));
    }
}
