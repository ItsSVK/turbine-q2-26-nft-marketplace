use litesvm::LiteSVM;
use solana_sdk::{
    account::Account,
    hash::Hash,
    instruction::{AccountMeta, Instruction},
    pubkey,
    signature::{Keypair, Signer},
    system_program,
    transaction::Transaction,
};

// Re-export so `use super::helpers::*` gives all common types.
pub use solana_sdk::pubkey::Pubkey;

// ── Program IDs ──────────────────────────────────────────────────────────────

pub const PROGRAM_ID: Pubkey =
    pubkey!("GdpmNmSGvz9AkftRpjdCXKeRroQie68vzUufvZTyCy8V");
pub const MPL_CORE_ID: Pubkey =
    pubkey!("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d");
pub const TOKEN_PROGRAM_ID: Pubkey =
    pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const TOKEN_2022_ID: Pubkey =
    pubkey!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
pub const ATA_PROGRAM_ID: Pubkey =
    pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bD");

// ── PDA seeds (mirror constants.rs) ─────────────────────────────────────────

const MARKETPLACE_SEED: &[u8] = b"marketplace";
const TREASURY_SEED: &[u8] = b"treasury";
const LISTING_SEED: &[u8] = b"listing";
const REWARDS_SEED: &[u8] = b"rewards";
const OFFER_SEED: &[u8] = b"offer";
const OFFER_VAULT_SEED: &[u8] = b"offer_vault";

// ── Anchor discriminator ─────────────────────────────────────────────────────

pub fn anchor_disc(name: &str) -> [u8; 8] {
    use solana_sdk::hash::hashv;
    let preimage = format!("global:{}", name);
    let hash = hashv(&[preimage.as_bytes()]);
    hash.to_bytes()[..8].try_into().unwrap()
}

// ── PDA derivation helpers ───────────────────────────────────────────────────

pub fn marketplace_pda(name: &str) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MARKETPLACE_SEED, name.as_bytes()], &PROGRAM_ID)
}

pub fn treasury_pda(marketplace: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TREASURY_SEED, marketplace.as_ref()], &PROGRAM_ID)
}

pub fn rewards_mint_pda(marketplace: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[REWARDS_SEED, marketplace.as_ref()], &PROGRAM_ID)
}

pub fn listing_pda(asset: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[LISTING_SEED, asset.as_ref()], &PROGRAM_ID)
}

pub fn offer_pda(asset: &Pubkey, buyer: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[OFFER_SEED, asset.as_ref(), buyer.as_ref()],
        &PROGRAM_ID,
    )
}

pub fn offer_vault_pda(asset: &Pubkey, buyer: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[OFFER_VAULT_SEED, asset.as_ref(), buyer.as_ref()],
        &PROGRAM_ID,
    )
}

// ── Instruction data (borsh-compatible manual encoding) ─────────────────────

fn encode_str(s: &str) -> Vec<u8> {
    let mut out = (s.len() as u32).to_le_bytes().to_vec();
    out.extend_from_slice(s.as_bytes());
    out
}

pub fn ix_init(name: &str, fee: u16) -> Vec<u8> {
    let mut d = anchor_disc("init").to_vec();
    d.extend_from_slice(&encode_str(name));
    d.extend_from_slice(&fee.to_le_bytes());
    d
}

pub fn ix_claim_sol_fee(amount: u64) -> Vec<u8> {
    let mut d = anchor_disc("claim_sol_fee").to_vec();
    d.extend_from_slice(&amount.to_le_bytes());
    d
}

pub fn ix_claim_token_fee(amount: u64) -> Vec<u8> {
    let mut d = anchor_disc("claim_token_fee").to_vec();
    d.extend_from_slice(&amount.to_le_bytes());
    d
}

pub fn ix_list(price: u64) -> Vec<u8> {
    let mut d = anchor_disc("list").to_vec();
    d.extend_from_slice(&price.to_le_bytes());
    d
}

pub fn ix_delist() -> Vec<u8> {
    anchor_disc("delist").to_vec()
}

pub fn ix_buy_with_sol() -> Vec<u8> {
    anchor_disc("buy_with_sol").to_vec()
}

pub fn ix_buy_with_token() -> Vec<u8> {
    anchor_disc("buy_with_token").to_vec()
}

pub fn ix_make_sol_offer(amount: u64) -> Vec<u8> {
    let mut d = anchor_disc("make_sol_offer").to_vec();
    d.extend_from_slice(&amount.to_le_bytes());
    d
}

pub fn ix_cancel_sol_offer() -> Vec<u8> {
    anchor_disc("cancel_sol_offer").to_vec()
}

pub fn ix_accept_sol_offer() -> Vec<u8> {
    anchor_disc("accept_sol_offer").to_vec()
}

pub fn ix_make_token_offer(amount: u64) -> Vec<u8> {
    let mut d = anchor_disc("make_token_offer").to_vec();
    d.extend_from_slice(&amount.to_le_bytes());
    d
}

pub fn ix_cancel_token_offer() -> Vec<u8> {
    anchor_disc("cancel_token_offer").to_vec()
}

pub fn ix_accept_token_offer() -> Vec<u8> {
    anchor_disc("accept_token_offer").to_vec()
}

// ── SVM setup ────────────────────────────────────────────────────────────────

pub fn program_binary_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/deploy/turbine_q2_26_nft_marketplace.so")
}

pub fn mpl_core_fixture_path() -> std::path::PathBuf {
    // scripts/fetch-test-programs.sh dumps the binary here.
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/deploy/mpl_core.so")
}

/// Returns an SVM with the marketplace program loaded.
/// Panics with a helpful message when the binary is missing (run `anchor build` first).
pub fn setup_svm() -> LiteSVM {
    let path = program_binary_path();
    let mut svm = LiteSVM::new();
    svm.add_program_from_file(PROGRAM_ID, &path).unwrap_or_else(|e| {
        panic!(
            "Failed to load program binary: {e}\n\
             Path: {}\n\
             Hint: run `anchor build` before running tests.",
            path.display()
        )
    });
    svm
}

/// Returns `Some(svm)` with both the marketplace program and mpl-core loaded,
/// or `None` (with an eprintln) when the mpl-core fixture is absent.
///
/// To obtain the fixture:
///   solana program dump CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d \
///     programs/turbine-q2-26-nft-marketplace/tests/fixtures/mpl_core.so \
///     -u mainnet-beta
pub fn setup_svm_with_mpl_core() -> Option<LiteSVM> {
    let mpl_path = mpl_core_fixture_path();
    if !mpl_path.exists() {
        eprintln!(
            "Skipping mpl-core test — binary not found at {}.\n\
             Run:  bash scripts/fetch-test-programs.sh",
            mpl_path.display()
        );
        return None;
    }
    let mut svm = setup_svm();
    svm.add_program_from_file(MPL_CORE_ID, &mpl_path)
        .expect("Failed to load mpl-core fixture");
    Some(svm)
}

// ── Account injection helpers ────────────────────────────────────────────────

/// Injects a minimal SPL-Token-2022 mint account directly into SVM state,
/// skipping the need for on-chain CPI calls during test setup.
pub fn inject_mint(svm: &mut LiteSVM, mint_key: &Pubkey, authority: &Pubkey, decimals: u8) {
    // SPL Token-2022 basic mint layout (82 bytes, no extensions).
    let mut data = vec![0u8; 82];
    data[0..4].copy_from_slice(&[1, 0, 0, 0]); // COption::Some tag
    data[4..36].copy_from_slice(authority.as_ref()); // mint_authority
    // supply = 0  (bytes 36..44, already zero)
    data[44] = decimals;
    data[45] = 1; // is_initialized = true
    // freeze_authority = COption::None (bytes 46..82, already zero)

    svm.set_account(
        *mint_key,
        Account {
            lamports: 1_461_600, // rent-exempt for 82 bytes
            data,
            owner: TOKEN_2022_ID,
            executable: false,
            rent_epoch: u64::MAX,
        },
    );
}

/// Injects a minimal SPL token account (ATA layout, 165 bytes) into SVM state.
pub fn inject_token_account(
    svm: &mut LiteSVM,
    ata_key: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
) {
    let mut data = vec![0u8; 165];
    data[0..32].copy_from_slice(mint.as_ref()); // mint
    data[32..64].copy_from_slice(owner.as_ref()); // owner
    data[64..72].copy_from_slice(&amount.to_le_bytes()); // amount
    // delegate: COption::None — bytes 72..76 already zero
    data[76] = 1; // state = Initialized
    // is_native: COption::None, delegated_amount = 0, close_authority: None — all zero

    svm.set_account(
        *ata_key,
        Account {
            lamports: 2_039_280, // rent-exempt for 165 bytes
            data,
            owner: TOKEN_2022_ID,
            executable: false,
            rent_epoch: u64::MAX,
        },
    );
}

// ── Convenience wrappers ─────────────────────────────────────────────────────

pub fn airdrop(svm: &mut LiteSVM, pubkey: &Pubkey, lamports: u64) {
    svm.airdrop(pubkey, lamports).unwrap();
}

pub fn blockhash(svm: &LiteSVM) -> Hash {
    svm.latest_blockhash()
}

/// Sends a single-instruction transaction signed by `signers[0]` as payer.
pub fn send_ix(svm: &mut LiteSVM, ix: Instruction, signers: &[&Keypair]) {
    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&signers[0].pubkey()),
        signers,
        blockhash(svm),
    );
    svm.send_transaction(tx)
        .unwrap_or_else(|e| panic!("transaction failed: {e:?}"));
}

// ── Composite fixture: initialised marketplace ───────────────────────────────

/// Initialises a fresh marketplace and returns
/// `(admin_keypair, marketplace_pda, treasury_pda, rewards_mint_pda)`.
pub fn fixture_marketplace(
    svm: &mut LiteSVM,
    name: &str,
    fee: u16,
) -> (Keypair, Pubkey, Pubkey, Pubkey) {
    let admin = Keypair::new();
    airdrop(svm, &admin.pubkey(), 10_000_000_000);

    let (marketplace, _) = marketplace_pda(name);
    let (treasury, _) = treasury_pda(&marketplace);
    let (rewards_mint, _) = rewards_mint_pda(&marketplace);

    let ix = Instruction {
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
    };

    send_ix(svm, ix, &[&admin]);
    (admin, marketplace, treasury, rewards_mint)
}
