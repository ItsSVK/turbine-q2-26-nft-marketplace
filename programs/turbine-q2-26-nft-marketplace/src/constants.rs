use anchor_lang::prelude::*;

#[constant]
pub const MARKETPLACE: &[u8] = b"marketplace";

#[constant]
pub const TREASURY: &[u8] = b"treasury";

#[constant]
pub const LISTING: &[u8] = b"listing";

#[constant]
pub const REWARDS_MINT: &[u8] = b"rewards";

#[constant]
pub const OFFER: &[u8] = b"offer";

#[constant]
pub const OFFER_VAULT: &[u8] = b"offer_vault";

#[constant]
pub const MINT_DECIMALS: u8 = 6;

#[constant]
pub const MAX_NAME_LENGTH: u8 = 30;
