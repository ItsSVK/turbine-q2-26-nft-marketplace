use super::helpers::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn init_ix(admin: &Keypair, name: &str, fee: u16) -> Instruction {
    let (marketplace, _) = marketplace_pda(name);
    let (treasury, _) = treasury_pda(&marketplace);
    let (rewards_mint, _) = rewards_mint_pda(&marketplace);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(admin.pubkey(), true),
            AccountMeta::new(marketplace, false),
            AccountMeta::new_readonly(treasury, false),
            AccountMeta::new(rewards_mint, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(TOKEN_2022_ID, false),
        ],
        data: ix_init(name, fee),
    }
}

fn send_init(name: &str, fee: u16) -> (litesvm::LiteSVM, Keypair, bool) {
    let mut svm = setup_svm();
    let admin = Keypair::new();
    airdrop(&mut svm, &admin.pubkey(), 10_000_000_000);

    let ix = init_ix(&admin, name, fee);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash(&svm),
    );
    let ok = svm.send_transaction(tx).is_ok();
    (svm, admin, ok)
}

// ── tests ────────────────────────────────────────────────────────────────────

#[test]
fn test_init_success() {
    let (svm, _admin, ok) = send_init("TestMarket", 250);
    assert!(ok, "init should succeed with valid name and fee");

    let (marketplace_key, _) = marketplace_pda("TestMarket");
    let account = svm
        .get_account(&marketplace_key)
        .expect("marketplace account must exist after init");
    assert!(
        account.data.len() > 8,
        "marketplace account should contain data beyond the 8-byte discriminator"
    );
}

#[test]
fn test_init_zero_fee_allowed() {
    let (_svm, _admin, ok) = send_init("ZeroFeeMarket", 0);
    assert!(ok, "fee = 0 (no-fee marketplace) should be valid");
}

#[test]
fn test_init_max_valid_fee() {
    // MAX_FEE_BASIS_POINTS = 10_000; the constraint is fee < 10_000
    let (_svm, _admin, ok) = send_init("AlmostMaxFeeMarket", 9_999);
    assert!(ok, "fee = 9999 is the largest valid fee");
}

#[test]
fn test_init_fee_equals_max_basis_points_fails() {
    let (_svm, _admin, ok) = send_init("MaxFeeMarket", 10_000);
    assert!(!ok, "fee = 10_000 should be rejected (must be strictly less)");
}

#[test]
fn test_init_empty_name_fails() {
    // trim().is_empty() catches whitespace-only strings
    let (_svm, _admin, ok) = send_init("   ", 250);
    assert!(!ok, "whitespace-only name should be rejected");
}

#[test]
fn test_init_name_too_long_fails() {
    let long_name = "a".repeat(31); // MAX_NAME_LENGTH = 30
    let (_svm, _admin, ok) = send_init(&long_name, 250);
    assert!(!ok, "name longer than 30 chars should be rejected");
}

#[test]
fn test_init_name_at_max_length_succeeds() {
    let max_name = "b".repeat(30);
    let (_svm, _admin, ok) = send_init(&max_name, 250);
    assert!(ok, "name of exactly 30 chars should be accepted");
}

#[test]
fn test_init_duplicate_name_fails() {
    let mut svm = setup_svm();
    let admin = Keypair::new();
    airdrop(&mut svm, &admin.pubkey(), 10_000_000_000);

    let name = "UniqueMarket";

    // First init – must succeed
    let ix1 = init_ix(&admin, name, 100);
    let tx1 = Transaction::new_signed_with_payer(
        &[ix1],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash(&svm),
    );
    svm.send_transaction(tx1).expect("first init should succeed");

    // Second init with the same name – marketplace PDA already exists, must fail
    let ix2 = init_ix(&admin, name, 100);
    let tx2 = Transaction::new_signed_with_payer(
        &[ix2],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash(&svm),
    );
    assert!(
        svm.send_transaction(tx2).is_err(),
        "initialising the same marketplace twice should fail"
    );
}

#[test]
fn test_init_multiple_distinct_marketplaces() {
    let mut svm = setup_svm();
    let admin = Keypair::new();
    airdrop(&mut svm, &admin.pubkey(), 50_000_000_000);

    for (name, fee) in [("Market1", 100u16), ("Market2", 200), ("Market3", 300)] {
        let ix = init_ix(&admin, name, fee);
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&admin.pubkey()),
            &[&admin],
            blockhash(&svm),
        );
        assert!(
            svm.send_transaction(tx).is_ok(),
            "init should succeed for marketplace '{name}'"
        );

        let (mp_key, _) = marketplace_pda(name);
        assert!(
            svm.get_account(&mp_key).is_some(),
            "marketplace PDA should exist for '{name}'"
        );
    }
}
