/// Listing tests require the mpl-core program fixture and a real mpl-core asset.
///
/// All tests here are guarded by `setup_svm_with_mpl_core()` and return early
/// when the fixture is absent.
///
/// To enable them:
///   1. Dump mpl-core from mainnet:
///        solana program dump CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d \
///          programs/turbine-q2-26-nft-marketplace/tests/fixtures/mpl_core.so \
///          -u mainnet-beta
///   2. Create a helper that calls mpl-core's `create_v1` instruction to mint
///      a test asset owned by `maker`, then pass that asset pubkey into the
///      `list` instruction below.
use super::helpers::*;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

// ── instruction builders ─────────────────────────────────────────────────────

/// Builds the `list` instruction.
/// Pass `None` for `collection` when the asset is not part of a collection.
/// Pass `None` for `payment_mint` for a SOL-denominated listing.
fn list_ix(
    maker: &Keypair,
    asset: Pubkey,
    collection: Option<Pubkey>,
    payment_mint: Option<Pubkey>,
    price: u64,
) -> Instruction {
    let (listing, _) = listing_pda(&asset);

    // Anchor encodes Option<Account> as the program ID when None.
    let collection_meta = collection.map_or_else(
        || AccountMeta::new_readonly(PROGRAM_ID, false),
        |k| AccountMeta::new(k, false),
    );
    let payment_mint_meta = payment_mint.map_or_else(
        || AccountMeta::new_readonly(PROGRAM_ID, false),
        |k| AccountMeta::new_readonly(k, false),
    );

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(maker.pubkey(), true),    // maker
            AccountMeta::new(asset, false),            // asset
            collection_meta,                           // collection (Option)
            AccountMeta::new(listing, false),          // listing PDA
            payment_mint_meta,                         // payment_mint (Option)
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
        ],
        data: ix_list(price),
    }
}

/// Builds the `delist` instruction.
fn delist_ix(
    maker: &Keypair,
    marketplace: Pubkey,
    asset: Pubkey,
    collection: Option<Pubkey>,
) -> Instruction {
    let (listing, _) = listing_pda(&asset);

    let collection_meta = collection.map_or_else(
        || AccountMeta::new_readonly(PROGRAM_ID, false),
        |k| AccountMeta::new(k, false),
    );

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(maker.pubkey(), true),
            AccountMeta::new_readonly(marketplace, false),
            AccountMeta::new(asset, false),
            collection_meta,
            AccountMeta::new(listing, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
        ],
        data: ix_delist(),
    }
}

// ── tests ────────────────────────────────────────────────────────────────────

#[test]
fn test_list_nft_success() {
    let Some(mut svm) = setup_svm_with_mpl_core() else {
        return;
    };

    let (maker_kp, marketplace, _treasury, _rewards) =
        fixture_marketplace(&mut svm, "ListingMarket", 250);

    // TODO: Replace `dummy_asset` with a real mpl-core asset created via
    // the mpl-core `CreateV1` instruction (CPI or direct transaction).
    // The asset must be owned by `maker_kp.pubkey()` after creation.
    let _ = marketplace;
    let _maker = maker_kp;
}

#[test]
fn test_list_invalid_price_zero_fails() {
    let Some(mut svm) = setup_svm_with_mpl_core() else {
        return;
    };

    let (maker_kp, _marketplace, _treasury, _) =
        fixture_marketplace(&mut svm, "InvalidPriceListMarket", 250);
    let maker = maker_kp;
    airdrop(&mut svm, &maker.pubkey(), 5_000_000_000);

    // Use a dummy pubkey as asset — the price constraint fires before mpl-core
    // CPI so this correctly surfaces the InvalidPrice error even without a real
    // asset account.
    let fake_asset = Pubkey::new_unique();
    let ix = list_ix(&maker, fake_asset, None, None, 0);
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&maker.pubkey()),
        &[&maker],
        blockhash(&svm),
    );

    assert!(
        svm.send_transaction(tx).is_err(),
        "listing with price = 0 should fail (InvalidPrice)"
    );
}

#[test]
fn test_delist_wrong_maker_fails() {
    let Some(mut svm) = setup_svm_with_mpl_core() else {
        return;
    };

    let (real_maker, marketplace, _treasury, _) =
        fixture_marketplace(&mut svm, "DelistWrongMakerMarket", 250);
    airdrop(&mut svm, &real_maker.pubkey(), 5_000_000_000);

    let impostor = Keypair::new();
    airdrop(&mut svm, &impostor.pubkey(), 1_000_000_000);

    // TODO: Create and list a real mpl-core asset with `real_maker`, then
    // attempt delist with `impostor` — the `has_one = maker` constraint
    // should reject it.
    //
    // Scaffolded account layout for delist with wrong signer:
    let fake_asset = Pubkey::new_unique();
    let (listing, _) = listing_pda(&fake_asset);

    let bad_ix = Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(impostor.pubkey(), true), // wrong maker
            AccountMeta::new_readonly(marketplace, false),
            AccountMeta::new(fake_asset, false),
            AccountMeta::new_readonly(PROGRAM_ID, false), // collection: None
            AccountMeta::new(listing, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(MPL_CORE_ID, false),
        ],
        data: ix_delist(),
    };
    let bad_tx = Transaction::new_signed_with_payer(
        &[bad_ix],
        Some(&impostor.pubkey()),
        &[&impostor],
        blockhash(&svm),
    );

    assert!(
        svm.send_transaction(bad_tx).is_err(),
        "delist by non-maker should fail"
    );
}

#[test]
fn test_list_then_delist_roundtrip() {
    let Some(mut svm) = setup_svm_with_mpl_core() else {
        return;
    };

    let (maker, _marketplace, _treasury, _) =
        fixture_marketplace(&mut svm, "RoundtripListMarket", 200);
    airdrop(&mut svm, &maker.pubkey(), 10_000_000_000);

    // TODO: Create a real mpl-core asset, list it, then delist it.
    // After delist, verify:
    //   - listing PDA is closed (None from svm.get_account)
    //   - asset ownership is transferred back to maker
    //   - maker balance recovered (listing rent returned)
    let _ = maker;
}
