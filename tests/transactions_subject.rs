//! Pure: the transaction subject.
//!
//! Every stored row and every dedupe key is relative to a subject -- the wallet a
//! decode is about. Filing a watched wallet's activity under our own address would
//! corrupt our history and our dedupe, so the subject has to survive every
//! conversion intact.

mod common;

use screenerbot::transactions::Subject;
use solana_sdk::pubkey::Pubkey;

#[test]
fn address_matches_the_pubkeys_base58_string() {
    let pubkey = Pubkey::new_unique();
    let subject = Subject::from(pubkey);

    assert_eq!(subject.address(), pubkey.to_string());
}

#[test]
fn pubkey_round_trips_the_original() {
    let pubkey = Pubkey::new_unique();
    let subject = Subject::from(pubkey);

    assert_eq!(subject.pubkey(), pubkey);
}

#[test]
fn subjects_from_the_same_pubkey_are_equal() {
    let pubkey = Pubkey::new_unique();

    assert_eq!(Subject::from(pubkey), Subject::from(pubkey));
}

#[test]
fn subjects_from_different_pubkeys_are_not_equal() {
    let a = Pubkey::new_unique();
    let b = Pubkey::new_unique();

    assert_ne!(Subject::from(a), Subject::from(b));
}

#[test]
fn display_renders_the_base58_address() {
    let pubkey = Pubkey::new_unique();
    let subject = Subject::from(pubkey);

    assert_eq!(subject.to_string(), pubkey.to_string());
}
