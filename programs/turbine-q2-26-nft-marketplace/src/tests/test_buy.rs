/// Buy tests require the mpl-core program fixture, a real mpl-core asset, and
/// an active listing on the marketplace.
///
/// Guard pattern: each test calls `setup_svm_with_mpl_core()` and returns early
/// if the fixture is absent.  See test_listing.rs for fixture setup instructions.
use super::helpers::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

// ── instruction builders ─────────────────────────────────────────────────────

/// Builds the `buy_with_sol` instruction.
/// Accounts mirror BuyWithSol in buy_with_sol.rs.
#[allow(dead_code)]
fn buy_with_sol_ix(
    taker: &Keypair,
    maker: Pubkey,
    marketplace: Pubkey,
    asset: Pubkey,
    collection: Option<Pubkey>,
    listing: Pubkey,
    treasury: Pubkey,
    rewards_mint: Pubkey,
    taker_reward_ata: Pubkey,
) -> Instruction {
    let collection_meta = collection.map_or_else(
        || AccountMeta::new_readonly(PROGRAM_ID, false),
        |k| AccountMeta::new(k, false),
    );

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(taker.pubkey(), true),       // taker (signer)
            AccountMeta::new(maker, false),               // maker (receives payment)
            AccountMeta::new_readonly(marketplace, false),
            AccountMeta::new(asset, false),
            collection_meta,
            AccountMeta::new(listing, false),
            AccountMeta::new(treasury, false),
            AccountMeta::new(rewards_mint, false),
            AccountMeta::new(taker_reward_ata, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(TOKEN_2022_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
        ],
        data: ix_buy_with_sol(),
    }
}

/// Builds the `buy_with_token` instruction.
/// Accounts mirror BuyWithToken in buy_with_token.rs.
#[allow(dead_code)]
fn buy_with_token_ix(
    taker: &Keypair,
    maker: Pubkey,
    marketplace: Pubkey,
    treasury: Pubkey,
    asset: Pubkey,
    collection: Option<Pubkey>,
    listing: Pubkey,
    payment_mint: Pubkey,
    rewards_mint: Pubkey,
    taker_payment_ata: Pubkey,
    maker_payment_ata: Pubkey,
    treasury_payment_ata: Pubkey,
    taker_reward_ata: Pubkey,
) -> Instruction {
    let collection_meta = collection.map_or_else(
        || AccountMeta::new_readonly(PROGRAM_ID, false),
        |k| AccountMeta::new(k, false),
    );

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(taker.pubkey(), true),
            AccountMeta::new(maker, false),
            AccountMeta::new_readonly(marketplace, false),
            AccountMeta::new_readonly(treasury, false),
            AccountMeta::new(asset, false),
            collection_meta,
            AccountMeta::new(listing, false),
            AccountMeta::new_readonly(payment_mint, false),
            AccountMeta::new(rewards_mint, false),
            AccountMeta::new(taker_payment_ata, false),
            AccountMeta::new(maker_payment_ata, false),
            AccountMeta::new(treasury_payment_ata, false),
            AccountMeta::new(taker_reward_ata, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(TOKEN_2022_ID, false),
            AccountMeta::new_readonly(ATA_PROGRAM_ID, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
        ],
        data: ix_buy_with_token(),
    }
}

// ── tests ────────────────────────────────────────────────────────────────────

/// Happy-path: taker buys a listed NFT with SOL.
/// Verifies:
///   - listing PDA is closed after purchase
///   - maker receives `price * (1 - fee/10_000)` lamports
///   - treasury receives `price * fee/10_000` lamports
///   - taker receives reward tokens equal to the listing price
#[test]
fn test_buy_with_sol_success() {
    let Some(mut svm) = setup_svm_with_mpl_core() else {
        return;
    };

    let fee_bps: u16 = 500; // 5 %
    let (maker, marketplace, treasury, rewards_mint) =
        fixture_marketplace(&mut svm, "BuyWithSolMarket", fee_bps);

    let taker = Keypair::new();
    airdrop(&mut svm, &maker.pubkey(), 5_000_000_000);
    airdrop(&mut svm, &taker.pubkey(), 5_000_000_000);

    // TODO:
    //   1. Create a mpl-core asset owned by `maker`.
    //   2. List it via the `list` instruction at 1 SOL price.
    //   3. Derive `taker_reward_ata` (ATA of taker for rewards_mint).
    //   4. Call buy_with_sol_ix and send the transaction.
    //   5. Assert balances and account states.
    let _ = (maker, marketplace, treasury, rewards_mint, taker);
}

/// Taker tries to buy with insufficient SOL balance.
#[test]
fn test_buy_with_sol_insufficient_balance_fails() {
    let Some(mut svm) = setup_svm_with_mpl_core() else {
        return;
    };

    let (_maker, _marketplace, _treasury, _rewards_mint) =
        fixture_marketplace(&mut svm, "BuyWithSolInsufficientMarket", 250);

    let taker = Keypair::new();
    airdrop(&mut svm, &taker.pubkey(), 1_000); // much less than a 1 SOL listing

    // TODO: Create and list an NFT at 1 SOL, then attempt buy_with_sol with
    //       `taker` who only has 1_000 lamports — should fail.
    let _ = taker;
}

/// Happy-path: taker buys a listed NFT with an SPL token.
/// Verifies:
///   - listing PDA is closed
///   - maker_payment_ata receives `amount * (1 - fee/10_000)` tokens
///   - treasury_payment_ata receives `amount * fee/10_000` tokens
///   - taker receives reward tokens equal to the listing price
#[test]
fn test_buy_with_token_success() {
    let Some(mut svm) = setup_svm_with_mpl_core() else {
        return;
    };

    let fee_bps: u16 = 250;
    let (_maker, _marketplace, _treasury, _rewards_mint) =
        fixture_marketplace(&mut svm, "BuyWithTokenMarket", fee_bps);

    let _taker = Keypair::new();

    // TODO:
    //   1. Create payment mint via inject_mint.
    //   2. Create taker ATA with sufficient token balance via inject_token_account.
    //   3. Create mpl-core asset owned by maker, list it with payment_mint.
    //   4. Call buy_with_token_ix and verify balances.
}

/// Taker tries to buy a token-denominated listing but provides the wrong payment mint.
#[test]
fn test_buy_with_token_wrong_mint_fails() {
    let Some(mut svm) = setup_svm_with_mpl_core() else {
        return;
    };

    let (_maker, _marketplace, _treasury, _rewards_mint) =
        fixture_marketplace(&mut svm, "WrongMintMarket", 250);

    // TODO: List asset with mint_A, attempt buy_with_token with mint_B.
    //       The constraint `listing.payment_mint == Some(payment_mint.key())`
    //       should reject the transaction.
}
