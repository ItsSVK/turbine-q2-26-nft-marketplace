/// SOL offer lifecycle tests (make + cancel) run without mpl-core because the
/// `asset` account is an unchecked `AccountInfo` used only as a PDA seed.
/// Tests for `accept_sol_offer`, `make_token_offer`, `cancel_token_offer`, and
/// `accept_token_offer` require additional fixtures and are guarded accordingly.
use super::helpers::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

// ── instruction builders ─────────────────────────────────────────────────────

fn make_sol_offer_ix(buyer: &Keypair, asset: Pubkey, amount: u64) -> Instruction {
    let (offer, _) = offer_pda(&asset, &buyer.pubkey());
    let (vault, _) = offer_vault_pda(&asset, &buyer.pubkey());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(buyer.pubkey(), true),
            AccountMeta::new_readonly(asset, false), // unchecked, any pubkey
            AccountMeta::new(offer, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: ix_make_sol_offer(amount),
    }
}

fn cancel_sol_offer_ix(buyer: &Keypair, asset: Pubkey) -> Instruction {
    let (offer, _) = offer_pda(&asset, &buyer.pubkey());
    let (vault, _) = offer_vault_pda(&asset, &buyer.pubkey());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(buyer.pubkey(), true),
            AccountMeta::new_readonly(asset, false),
            AccountMeta::new(offer, false),
            AccountMeta::new(vault, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: ix_cancel_sol_offer(),
    }
}

// ── make_sol_offer ───────────────────────────────────────────────────────────

#[test]
fn test_make_sol_offer_success() {
    let mut svm = setup_svm();
    let buyer = Keypair::new();
    airdrop(&mut svm, &buyer.pubkey(), 5_000_000_000);

    let asset = Pubkey::new_unique();
    let offer_amount: u64 = 1_000_000_000; // 1 SOL

    let (vault, _) = offer_vault_pda(&asset, &buyer.pubkey());
    let vault_before = svm.get_account(&vault).map_or(0, |a| a.lamports);

    let ix = make_sol_offer_ix(&buyer, asset, offer_amount);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&buyer.pubkey()),
        &[&buyer],
        blockhash(&svm),
    );
    svm.send_transaction(tx).expect("make_sol_offer should succeed");

    // Verify the offer account was created.
    let (offer_key, _) = offer_pda(&asset, &buyer.pubkey());
    assert!(
        svm.get_account(&offer_key).is_some(),
        "offer PDA should exist after make_sol_offer"
    );

    // Verify SOL was moved into the vault.
    let vault_after = svm.get_account(&vault).map_or(0, |a| a.lamports);
    assert!(
        vault_after > vault_before,
        "vault balance should increase by the offer amount"
    );
    assert!(
        vault_after >= offer_amount,
        "vault should hold at least the offered amount"
    );
}

#[test]
fn test_make_sol_offer_zero_amount_fails() {
    let mut svm = setup_svm();
    let buyer = Keypair::new();
    airdrop(&mut svm, &buyer.pubkey(), 2_000_000_000);

    let asset = Pubkey::new_unique();
    let ix = make_sol_offer_ix(&buyer, asset, 0);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&buyer.pubkey()),
        &[&buyer],
        blockhash(&svm),
    );

    assert!(
        svm.send_transaction(tx).is_err(),
        "offering 0 lamports should be rejected (InvalidPrice)"
    );
}

#[test]
fn test_make_sol_offer_insufficient_balance_fails() {
    let mut svm = setup_svm();
    let buyer = Keypair::new();
    // Give barely enough for rent but not the offer amount.
    airdrop(&mut svm, &buyer.pubkey(), 10_000);

    let asset = Pubkey::new_unique();
    let ix = make_sol_offer_ix(&buyer, asset, 2_000_000_000); // 2 SOL, buyer has ~0
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&buyer.pubkey()),
        &[&buyer],
        blockhash(&svm),
    );

    assert!(
        svm.send_transaction(tx).is_err(),
        "offer should fail when buyer has insufficient balance"
    );
}

// ── cancel_sol_offer ─────────────────────────────────────────────────────────

#[test]
fn test_cancel_sol_offer_success() {
    let mut svm = setup_svm();
    let buyer = Keypair::new();
    airdrop(&mut svm, &buyer.pubkey(), 5_000_000_000);

    let asset = Pubkey::new_unique();
    let offer_amount: u64 = 500_000_000; // 0.5 SOL

    // First make an offer.
    let make_ix = make_sol_offer_ix(&buyer, asset, offer_amount);
    let make_tx = Transaction::new_signed_with_payer(
        &[make_ix],
        Some(&buyer.pubkey()),
        &[&buyer],
        blockhash(&svm),
    );
    svm.send_transaction(make_tx).expect("make_sol_offer should succeed");

    let buyer_after_offer = svm.get_account(&buyer.pubkey()).map_or(0, |a| a.lamports);

    // Now cancel the offer.
    let cancel_ix = cancel_sol_offer_ix(&buyer, asset);
    let cancel_tx = Transaction::new_signed_with_payer(
        &[cancel_ix],
        Some(&buyer.pubkey()),
        &[&buyer],
        blockhash(&svm),
    );
    svm.send_transaction(cancel_tx).expect("cancel_sol_offer should succeed");

    // Anchor's `close = buyer` zeroes the offer account's lamports and data.
    let (offer_key, _) = offer_pda(&asset, &buyer.pubkey());
    let offer_account = svm.get_account(&offer_key);
    assert!(
        offer_account.map_or(true, |a| a.lamports == 0 && a.data.is_empty()),
        "offer PDA should be closed (0 lamports, empty data) after cancel"
    );

    // Buyer should recover the locked SOL (minus tx fees).
    let buyer_after_cancel = svm.get_account(&buyer.pubkey()).map_or(0, |a| a.lamports);
    assert!(
        buyer_after_cancel > buyer_after_offer,
        "buyer's balance should increase after cancel (vault + offer rent returned)"
    );
}

#[test]
fn test_cancel_sol_offer_wrong_buyer_fails() {
    let mut svm = setup_svm();
    let buyer = Keypair::new();
    airdrop(&mut svm, &buyer.pubkey(), 5_000_000_000);

    let asset = Pubkey::new_unique();

    let make_ix = make_sol_offer_ix(&buyer, asset, 1_000_000_000);
    let make_tx = Transaction::new_signed_with_payer(
        &[make_ix],
        Some(&buyer.pubkey()),
        &[&buyer],
        blockhash(&svm),
    );
    svm.send_transaction(make_tx).expect("make_sol_offer should succeed");

    // Different keypair attempts to cancel.
    let thief = Keypair::new();
    airdrop(&mut svm, &thief.pubkey(), 1_000_000_000);

    let (offer_key, _) = offer_pda(&asset, &buyer.pubkey());
    let (vault_key, _) = offer_vault_pda(&asset, &buyer.pubkey());

    let bad_cancel_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(thief.pubkey(), true), // wrong signer
            AccountMeta::new_readonly(asset, false),
            AccountMeta::new(offer_key, false),
            AccountMeta::new(vault_key, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: ix_cancel_sol_offer(),
    };
    let bad_tx = Transaction::new_signed_with_payer(
        &[bad_cancel_ix],
        Some(&thief.pubkey()),
        &[&thief],
        blockhash(&svm),
    );

    assert!(
        svm.send_transaction(bad_tx).is_err(),
        "a non-buyer should not be able to cancel the offer (has_one = buyer)"
    );
}

#[test]
fn test_make_and_cancel_multiple_offers_different_assets() {
    let mut svm = setup_svm();
    let buyer = Keypair::new();
    airdrop(&mut svm, &buyer.pubkey(), 20_000_000_000);

    let assets: Vec<Pubkey> = (0..3).map(|_| Pubkey::new_unique()).collect();
    let amount: u64 = 1_000_000_000;

    // Place an offer on each asset.
    for &asset in &assets {
        let ix = make_sol_offer_ix(&buyer, asset, amount);
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&buyer.pubkey()),
            &[&buyer],
            blockhash(&svm),
        );
        svm.send_transaction(tx)
            .expect("make_sol_offer should succeed for each distinct asset");
    }

    // Cancel each offer.
    for &asset in &assets {
        let ix = cancel_sol_offer_ix(&buyer, asset);
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&buyer.pubkey()),
            &[&buyer],
            blockhash(&svm),
        );
        svm.send_transaction(tx)
            .expect("cancel_sol_offer should succeed for each offer");

        let (offer_key, _) = offer_pda(&asset, &buyer.pubkey());
        let offer_account = svm.get_account(&offer_key);
        assert!(
            offer_account.map_or(true, |a| a.lamports == 0 && a.data.is_empty()),
            "offer PDA for asset should be closed (0 lamports, empty data) after cancel"
        );
    }
}

// ── accept_sol_offer (requires mpl-core + listed NFT) ────────────────────────

/// Full acceptance test requires:
///   1. mpl-core fixture at tests/fixtures/mpl_core.so
///   2. A real mpl-core asset created via mpl-core's `create` instruction
///   3. The asset to be listed on the marketplace
///
/// Obtain the fixture with:
///   solana program dump CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d \
///     programs/turbine-q2-26-nft-marketplace/tests/fixtures/mpl_core.so -u mainnet-beta
#[test]
fn test_accept_sol_offer_requires_mpl_core() {
    let Some(_svm) = setup_svm_with_mpl_core() else {
        return; // skipped — fixture absent
    };
    // TODO: Create mpl-core asset, list it, make offer, then accept.
    // The scaffolding below mirrors the required accounts from AcceptSolOffer:
    //
    //   AccountMeta::new(maker.pubkey(), true),
    //   AccountMeta::new(buyer.pubkey(), false),
    //   AccountMeta::new_readonly(marketplace, false),
    //   AccountMeta::new(asset_key, false),
    //   AccountMeta::new_readonly(collection, false),   // Option
    //   AccountMeta::new(listing, false),
    //   AccountMeta::new(offer_key, false),
    //   AccountMeta::new(offer_vault, false),
    //   AccountMeta::new(treasury, false),
    //   AccountMeta::new_readonly(system_program::ID, false),
    //   AccountMeta::new_readonly(MPL_CORE_ID, false),
}

// ── token offer tests (require SPL token setup) ───────────────────────────────

/// Token offer tests need a funded SPL-Token-2022 mint and ATA.
/// Use `inject_mint` / `inject_token_account` from helpers, then call
/// `make_token_offer` and `cancel_token_offer`.
///
/// Skeleton for make_token_offer:
///
///   let mint = Pubkey::new_unique();
///   inject_mint(&mut svm, &mint, &buyer.pubkey(), 6);
///
///   let buyer_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
///       &buyer.pubkey(), &mint, &TOKEN_2022_ID
///   );
///   inject_token_account(&mut svm, &buyer_ata, &mint, &buyer.pubkey(), 1_000_000);
///
///   let (offer_key, _) = offer_pda(&asset, &buyer.pubkey());
///   let offer_vault_ata = spl_associated_token_account::get_associated_token_address_with_program_id(
///       &offer_key, &mint, &TOKEN_2022_ID
///   );
///
///   Instruction {
///       program_id: PROGRAM_ID,
///       accounts: vec![
///           AccountMeta::new(buyer.pubkey(), true),
///           AccountMeta::new_readonly(asset, false),
///           AccountMeta::new(offer_key, false),
///           AccountMeta::new_readonly(mint, false),
///           AccountMeta::new(buyer_ata, false),
///           AccountMeta::new(offer_vault_ata, false),
///           AccountMeta::new_readonly(TOKEN_2022_ID, false),
///           AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
///           AccountMeta::new_readonly(system_program::ID, false),
///       ],
///       data: ix_make_token_offer(1_000_000),
///   }
#[test]
fn test_make_token_offer_scaffold() {
    // Placeholder — implement once spl-associated-token-account is available
    // in dev-dependencies for ATA address derivation.
}
