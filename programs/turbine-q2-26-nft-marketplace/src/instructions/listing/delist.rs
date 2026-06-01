use crate::{Listing, MarketPlace, MplAsset, MplCollection, MplCore, LISTING, MARKETPLACE};
use anchor_lang::prelude::*;
use mpl_core::instructions::TransferV1CpiBuilder;

#[derive(Accounts)]
pub struct Delist<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(
        seeds = [MARKETPLACE, marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Box<Account<'info, MarketPlace>>,

    #[account(mut)]
    pub asset: Box<Account<'info, MplAsset>>,

    pub collection: Option<Box<Account<'info, MplCollection>>>,

    #[account(
        mut,
        close = maker,
        seeds = [LISTING, asset.key().as_ref()],
        bump = listing.bump,
        has_one = maker,
        has_one = asset,
    )]
    pub listing: Account<'info, Listing>,

    pub system_program: Program<'info, System>,

    pub mpl_core_program: Program<'info, MplCore>,
}

impl<'info> Delist<'info> {
    pub fn cancel_listing(&mut self) -> Result<()> {
        let asset_key = self.asset.key();

        let signer_seeds: &[&[&[u8]]] = &[&[LISTING, asset_key.as_ref(), &[self.listing.bump]]];

        TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(
                self.collection
                    .as_deref()
                    .map(|collection| collection.as_ref()),
            )
            .payer(&self.maker.to_account_info())
            .authority(Some(&self.listing.to_account_info()))
            .new_owner(&self.maker.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke_signed(signer_seeds)?;

        Ok(())
    }
}
