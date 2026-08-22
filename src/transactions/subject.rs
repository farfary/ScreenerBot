//! The wallet a processing run is about — chain-neutral identity.

use crate::chains::{AccountId, ChainId};
use crate::errors::DataError;

/// The wallet a processing run is *about*.
///
/// Every decode, every stored row and every dedupe key is relative to a subject.
/// The bot's own trading wallet is simply the first subject; watched wallets are
/// the others. The database has always had a `wallet_address` column on every
/// table -- the subject is what finally makes that column mean something, instead
/// of every write silently resolving to the bot's own wallet.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Subject {
    account: AccountId,
}

impl Subject {
    /// Builds a subject from a chain-qualified account identity.
    pub fn from_account(account: AccountId) -> Self {
        Self { account }
    }

    /// The bot's own trading wallet.
    pub fn own() -> crate::Result<Self> {
        let address = crate::utils::get_wallet_address()?;
        let account = AccountId::new(crate::chains::active_chain(), address).map_err(|error| {
            crate::Error::Data(DataError::ValidationError {
                field: "wallet_address".to_owned(),
                value: String::new(),
                reason: error.to_string(),
            })
        })?;
        Ok(Self::from_account(account))
    }

    /// The chain-qualified account this subject represents.
    pub fn account(&self) -> &AccountId {
        &self.account
    }

    /// The subject's address as stored in the `wallet_address` column.
    pub fn address(&self) -> String {
        self.account.address().to_owned()
    }

    /// The blockchain this subject belongs to.
    pub fn chain(&self) -> ChainId {
        self.account.chain()
    }
}

impl std::fmt::Display for Subject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.account.address())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chains::AccountId;

    #[test]
    fn shared_subject_round_trips_chain_and_address_without_solana_types() {
        let account = AccountId::new(ChainId::Solana, "WalletAddress111").unwrap();
        let subject = Subject::from_account(account.clone());

        assert_eq!(subject.chain(), ChainId::Solana);
        assert_eq!(subject.address(), "WalletAddress111");
        assert_eq!(subject.account(), &account);
        assert_eq!(subject.to_string(), "WalletAddress111");
        assert_eq!(
            Subject::from_account(account.clone()),
            Subject::from_account(account)
        );
    }
}
