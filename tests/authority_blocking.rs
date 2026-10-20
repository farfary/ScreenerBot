// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! The blocked-address set rejects a token for a blocked authority or creator at H4, never blocks
//! the address a renounced authority holds, and lifts a creator's block when the data service
//! stops reporting it.

use crate::common::{filter_token, onchain_rules};
use screenerbot::chains::solana::constants::SYSTEM_PROGRAM_ID;
use screenerbot::chains::solana::solana_sdk::pubkey::Pubkey;
use screenerbot::chains::solana::solana_sdk::signature::Keypair;
use screenerbot::chains::solana::solana_sdk::signer::Signer;
use screenerbot::chains::ChainId;
use screenerbot::config::schemas::OnChainFilters;
use screenerbot::filtering::sources::FilterRejectionReason;
use screenerbot::tokens::authority_cache::{is_blocked_authority, refresh_blocked_from_db};
use screenerbot::tokens::creator_reputation::{apply_answer, CreatorsResponse, ServiceCreator};
use screenerbot::tokens::TokenDatabase;

const BLOCKED: &str = "BlockedAuthority1111111111111111111111111111";
const MINT: &str = "FilterMint111111111111111111111111111111111";
const PLATFORM_UPDATE_AUTHORITY: &str = "PlatformUpdateAuthority11111111111111111111";

fn listed(address: &str) -> ServiceCreator {
    ServiceCreator {
        address: address.to_owned(),
        tokens: 5,
        rugged: 5,
    }
}

fn list(creators: Vec<ServiceCreator>) -> CreatorsResponse {
    CreatorsResponse {
        ready: true,
        creators,
    }
}

/// The System Program is the update, freeze or mint authority of every token whose authority
/// was given up; a set (or a stored manual row) that names it must not reject those tokens,
/// while any other blocked address stays blocked.
#[test]
fn the_renounced_authority_is_never_blocked() {
    refresh_blocked_from_db(
        ChainId::Solana,
        vec![SYSTEM_PROGRAM_ID.to_owned(), BLOCKED.to_owned()],
    );

    assert!(!is_blocked_authority(ChainId::Solana, SYSTEM_PROGRAM_ID));
    assert!(is_blocked_authority(ChainId::Solana, BLOCKED));
}

/// A token created by a blocked wallet is rejected with its own reason, and the block lifts
/// at the sync that no longer lists the wallet. Invalid entries (the System Program, a derived
/// address) never reach the set, so tokens carrying them as creator pass, as do tokens with no
/// creator and tokens whose update authority is shared by a launch platform but not listed.
#[test]
fn a_blocked_creator_rejects_the_token_until_the_list_drops_it() {
    let directory = tempfile::tempdir().expect("temp dir");
    let path = directory.path().join("tokens.db");
    let db =
        TokenDatabase::new(&path.to_string_lossy(), ChainId::Solana).expect("open tokens store");
    let config = OnChainFilters::default();
    let wallet = Keypair::new().pubkey();
    let (derived, _) = Pubkey::find_program_address(&[b"creator"], &wallet);
    let wallet = wallet.to_string();
    let derived = derived.to_string();

    let with_creator = |creator: Option<&str>| {
        let mut token = filter_token(MINT);
        token.creator = creator.map(str::to_owned);
        token
    };

    let blocked = apply_answer(
        &db,
        Some(&list(vec![
            listed(&wallet),
            listed(SYSTEM_PROGRAM_ID),
            listed(&derived),
        ])),
        1_000,
    )
    .expect("sync the list");
    assert_eq!(blocked, 1, "only the personal wallet is stored");

    assert_eq!(
        onchain_rules(&with_creator(Some(&wallet)), &config),
        Err(FilterRejectionReason::OnChainKnownScamCreator)
    );
    for passing in [Some(SYSTEM_PROGRAM_ID), Some(derived.as_str()), None] {
        assert!(
            onchain_rules(&with_creator(passing), &config).is_ok(),
            "creator {passing:?} must not be rejected"
        );
    }
    let mut platform_token = with_creator(None);
    platform_token.update_authority = Some(PLATFORM_UPDATE_AUTHORITY.to_owned());
    assert!(onchain_rules(&platform_token, &config).is_ok());

    apply_answer(&db, Some(&list(Vec::new())), 2_000).expect("sync an empty list");

    assert!(onchain_rules(&with_creator(Some(&wallet)), &config).is_ok());
}
