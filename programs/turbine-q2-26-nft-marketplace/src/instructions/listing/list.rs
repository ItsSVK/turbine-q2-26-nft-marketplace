use crate::{error::MarketplaceError, Listing, MplAsset, MplCollection, MplCore, LISTING};
use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use mpl_core::instructions::TransferV1CpiBuilder;

#[derive(Accounts)]
pub struct List<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mut)]
    pub asset: Box<Account<'info, MplAsset>>,

    #[account(mut)]
    pub collection: Option<Box<Account<'info, MplCollection>>>,

    #[account(
        init,
        space = Listing::DISCRIMINATOR.len() + Listing::INIT_SPACE,
        payer = maker,
        seeds = [LISTING, asset.key().as_ref()],
        bump
    )]
    pub listing: Account<'info, Listing>,

    pub payment_mint: Option<InterfaceAccount<'info, Mint>>,

    pub system_program: Program<'info, System>,

    pub mpl_core_program: Program<'info, MplCore>,
}

impl<'info> List<'info> {
    pub fn create_listing(&mut self, price: u64, bumps: &ListBumps) -> Result<()> {
        require!(price > 0, MarketplaceError::InvalidPrice);

        self.listing.set_inner(Listing {
            maker: self.maker.key(),
            asset: self.asset.key(),
            price,
            bump: bumps.listing,
            payment_mint: self.payment_mint.as_ref().map(|mint| mint.key()),
        });

        TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(
                self.collection
                    .as_deref()
                    .map(|collection| collection.as_ref()),
            )
            .payer(&self.maker.to_account_info())
            .authority(Some(&self.maker.to_account_info()))
            .new_owner(&self.listing.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke()?;

        Ok(())
    }
}
