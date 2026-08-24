//! Pure: the transaction subject and subject-relative deltas.

mod common;

use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use screenerbot::chains::solana::transactions::subject as solana_subject;
use screenerbot::chains::{AccountId, ChainId, Error as ChainError};
use screenerbot::transactions::deltas::{DeltaKind, SubjectAssetDelta, NATIVE_SOL_SENTINEL};
use screenerbot::transactions::Subject;

#[test]
fn shared_subject_round_trips_chain_and_address_without_solana_sdk() {
    let account = AccountId::new(ChainId::Solana, "WalletAddress111").unwrap();
    let subject = Subject::from_account(account.clone());

    assert_eq!(subject.chain(), ChainId::Solana);
    assert_eq!(subject.address(), "WalletAddress111");
    assert_eq!(subject.account(), &account);
    assert_eq!(subject.to_string(), "WalletAddress111");
}

#[test]
fn solana_pubkey_round_trips_through_shared_subject() {
    let pubkey = Pubkey::new_unique();
    let subject = solana_subject::from_pubkey(pubkey);

    assert_eq!(subject.chain(), ChainId::Solana);
    assert_eq!(subject.address(), pubkey.to_string());
    assert_eq!(solana_subject::try_to_pubkey(&subject).unwrap(), pubkey);
    assert_eq!(
        solana_subject::try_from_address(&pubkey.to_string()).unwrap(),
        subject
    );
}

#[test]
fn invalid_or_wrong_chain_account_conversion_is_rejected() {
    assert!(matches!(
        solana_subject::try_from_address("not-a-valid-pubkey"),
        Err(ChainError::InvalidAccount { .. })
    ));
    assert!(matches!(
        "evm".parse::<ChainId>(),
        Err(ChainError::UnsupportedChain { .. })
    ));
    assert!(serde_json::from_str::<AccountId>(r#"{"chain":"evm","address":"0xabc"}"#).is_err());

    let invalid = AccountId::new(ChainId::Solana, "not-a-valid-pubkey").unwrap();
    assert!(matches!(
        solana_subject::try_from_account(invalid),
        Err(ChainError::InvalidAccount { .. })
    ));
}

#[test]
fn subject_asset_delta_carries_chain_and_raw_native_fee() {
    let delta = SubjectAssetDelta {
        chain: ChainId::Solana,
        wallet_address: "wallet".to_owned(),
        signature: "sig".to_owned(),
        mint: NATIVE_SOL_SENTINEL.to_owned(),
        slot: Some(42),
        block_time: Some(1_700_000_000),
        tx_index: 0,
        delta_raw: -1_000_000_000,
        before_raw: None,
        after_raw: None,
        decimals: 9,
        kind: DeltaKind::Trade,
        venue: Some("raydium".to_owned()),
        fee_native_raw: Some(5_000),
        success: true,
    };

    assert_eq!(delta.chain, ChainId::Solana);
    assert_eq!(delta.fee_native_raw, Some(5_000));
}
