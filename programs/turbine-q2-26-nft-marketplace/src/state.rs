use anchor_lang::prelude::*;
use mpl_core::ID as MPL_CORE_ID;

use crate::MAX_NAME_LENGTH;

pub use mpl_core::accounts::BaseAssetV1 as MplAsset;
pub use mpl_core::accounts::BaseCollectionV1 as MplCollection;

#[account]
#[derive(InitSpace)]
pub struct MarketPlace {
    pub admin: Pubkey,
    pub fee: u16,
    pub bump: u8,
    pub treasury_bump: u8,
    pub rewards_bump: u8,
    #[max_len(MAX_NAME_LENGTH)]
    pub name: String,
}

#[account]
#[derive(InitSpace)]
pub struct Listing {
    pub maker: Pubkey,
    pub asset: Pubkey,
    pub price: u64,
    pub bump: u8,
    pub payment_mint: Option<Pubkey>,
}

#[account]
#[derive(InitSpace)]
pub struct Offer {
    pub buyer: Pubkey,
    pub asset: Pubkey,
    pub amount: u64,
    pub bump: u8,
    pub vault_bump: Option<u8>,
    pub payment_mint: Option<Pubkey>,
}

#[derive(Clone)]
pub struct MplCore;

impl Id for MplCore {
    fn id() -> Pubkey {
        MPL_CORE_ID
    }
}
