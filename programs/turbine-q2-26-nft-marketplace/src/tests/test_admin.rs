use super::helpers::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn claim_sol_fee_ix(
    admin: &Keypair,
    marketplace: Pubkey,
    treasury: Pubkey,
    recipient: Pubkey,
    amount: u64,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(admin.pubkey(), true),
            AccountMeta::new_readonly(marketplace, false),
            AccountMeta::new(treasury, false),
            AccountMeta::new(recipient, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: ix_claim_sol_fee(amount),
    }
}

// ── tests: claim_sol_fee ─────────────────────────────────────────────────────

#[test]
fn test_claim_sol_fee_success() {
    let mut svm = setup_svm();
    let (admin, marketplace, treasury, _) = fixture_marketplace(&mut svm, "SolFeeMarket", 500);

    // Fund the treasury beyond rent-exemption so there is something to claim.
    let fee_amount: u64 = 5_000_000; // 0.005 SOL
    let rent_buffer: u64 = 1_000_000;
    airdrop(&mut svm, &treasury, fee_amount + rent_buffer);

    let recipient = Keypair::new();
    let recipient_before = svm
        .get_account(&recipient.pubkey())
        .map_or(0, |a| a.lamports);

    let ix = claim_sol_fee_ix(&admin, marketplace, treasury, recipient.pubkey(), fee_amount);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash(&svm),
    );

    svm.send_transaction(tx)
        .expect("claim_sol_fee should succeed when treasury has enough funds");

    let recipient_after = svm
        .get_account(&recipient.pubkey())
        .map_or(0, |a| a.lamports);
    assert_eq!(
        recipient_after - recipient_before,
        fee_amount,
        "recipient should receive exactly the claimed amount"
    );
}

#[test]
fn test_claim_sol_fee_zero_amount_fails() {
    let mut svm = setup_svm();
    let (admin, marketplace, treasury, _) = fixture_marketplace(&mut svm, "ZeroClaimMarket", 500);
    airdrop(&mut svm, &treasury, 2_000_000);

    let recipient = Keypair::new();
    let ix = claim_sol_fee_ix(&admin, marketplace, treasury, recipient.pubkey(), 0);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash(&svm),
    );

    assert!(
        svm.send_transaction(tx).is_err(),
        "claiming 0 lamports should be rejected (InvalidFee)"
    );
}

#[test]
fn test_claim_sol_fee_insufficient_treasury_fails() {
    let mut svm = setup_svm();
    let (admin, marketplace, treasury, _) =
        fixture_marketplace(&mut svm, "PoorTreasuryMarket", 500);
    // Deliberately do NOT add extra lamports – treasury only holds rent-exemption.

    let recipient = Keypair::new();
    let ix =
        claim_sol_fee_ix(&admin, marketplace, treasury, recipient.pubkey(), 5_000_000);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash(&svm),
    );

    assert!(
        svm.send_transaction(tx).is_err(),
        "claiming more than the spendable treasury balance should fail (InsufficientTreasuryFunds)"
    );
}

#[test]
fn test_claim_sol_fee_wrong_admin_fails() {
    let mut svm = setup_svm();
    let (_real_admin, marketplace, treasury, _) =
        fixture_marketplace(&mut svm, "WrongAdminMarket", 500);
    airdrop(&mut svm, &treasury, 10_000_000);

    let impostor = Keypair::new();
    airdrop(&mut svm, &impostor.pubkey(), 1_000_000_000);
    let recipient = Keypair::new();

    // `impostor` is a signer but `has_one = admin` on the marketplace will reject it.
    let ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(impostor.pubkey(), true), // wrong admin
            AccountMeta::new_readonly(marketplace, false),
            AccountMeta::new(treasury, false),
            AccountMeta::new(recipient.pubkey(), false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: ix_claim_sol_fee(1_000_000),
    };
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&impostor.pubkey()),
        &[&impostor],
        blockhash(&svm),
    );

    assert!(
        svm.send_transaction(tx).is_err(),
        "a non-admin signer should be rejected by has_one = admin"
    );
}

#[test]
fn test_claim_sol_fee_partial_then_full() {
    let mut svm = setup_svm();
    let (admin, marketplace, treasury, _) = fixture_marketplace(&mut svm, "PartialClaimMarket", 300);

    let total_fees: u64 = 20_000_000; // 0.02 SOL
    let rent_buffer: u64 = 2_000_000;
    airdrop(&mut svm, &treasury, total_fees + rent_buffer);

    let recipient = Keypair::new();

    // Claim half
    let half = total_fees / 2;
    let ix1 = claim_sol_fee_ix(&admin, marketplace, treasury, recipient.pubkey(), half);
    let tx1 = Transaction::new_signed_with_payer(
        &[ix1],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash(&svm),
    );
    svm.send_transaction(tx1).expect("first partial claim should succeed");

    // Expire blockhash so the second transaction is not a duplicate.
    svm.expire_blockhash();

    // Claim remainder (slightly different amount to also avoid data-level dedup)
    let ix2 = claim_sol_fee_ix(&admin, marketplace, treasury, recipient.pubkey(), half);
    let tx2 = Transaction::new_signed_with_payer(
        &[ix2],
        Some(&admin.pubkey()),
        &[&admin],
        blockhash(&svm),
    );
    svm.send_transaction(tx2).expect("second partial claim should succeed");

    let final_balance = svm
        .get_account(&recipient.pubkey())
        .map_or(0, |a| a.lamports);
    assert_eq!(
        final_balance, total_fees,
        "recipient should have received the full fee amount across two claims"
    );
}
