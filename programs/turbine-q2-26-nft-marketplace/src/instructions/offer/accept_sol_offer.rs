use crate::{
    error::MarketplaceError, Listing, MarketPlace, MplAsset, MplCollection, MplCore, Offer,
    LISTING, MARKETPLACE, OFFER, OFFER_VAULT, TREASURY,
};
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::token_2022::spl_token_2022::extension::transfer_fee::MAX_FEE_BASIS_POINTS;
use mpl_core::instructions::TransferV1CpiBuilder;

#[derive(Accounts)]
pub struct AcceptSolOffer<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mut)]
    pub buyer: SystemAccount<'info>,

    #[account(
        seeds = [MARKETPLACE, marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, MarketPlace>,

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

    #[account(
        mut,
        close = buyer,
        seeds = [OFFER, asset.key().as_ref(), buyer.key().as_ref()],
        bump = offer.bump,
        has_one = buyer,
        has_one = asset,
    )]
    pub offer: Account<'info, Offer>,

    #[account(
        mut,
        seeds = [OFFER_VAULT, asset.key().as_ref(), buyer.key().as_ref()],
        bump = offer.vault_bump.unwrap(),
    )]
    pub offer_vault: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [TREASURY, marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,

    pub system_program: Program<'info, System>,

    pub mpl_core_program: Program<'info, MplCore>,
}

impl<'info> AcceptSolOffer<'info> {
    pub fn send_sol(&mut self) -> Result<()> {
        let amount = self.offer.amount;

        require!(amount > 0, MarketplaceError::InvalidPrice);

        let fee = (amount as u128)
            .checked_mul(self.marketplace.fee as u128)
            .ok_or(MarketplaceError::MathOverflow)?
            .checked_div(MAX_FEE_BASIS_POINTS as u128)
            .ok_or(MarketplaceError::MathOverflow)? as u64;

        let maker_amount = amount
            .checked_sub(fee)
            .ok_or(MarketplaceError::MathOverflow)?;

        let asset_key = self.asset.key();
        let buyer_key = self.buyer.key();
        let vault_bump = [self.offer.vault_bump.unwrap()];

        let vault_signer_seeds: &[&[&[u8]]] = &[&[
            OFFER_VAULT,
            asset_key.as_ref(),
            buyer_key.as_ref(),
            &vault_bump,
        ]];

        transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.offer_vault.to_account_info(),
                    to: self.maker.to_account_info(),
                },
                vault_signer_seeds,
            ),
            maker_amount,
        )?;

        transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.offer_vault.to_account_info(),
                    to: self.treasury.to_account_info(),
                },
                vault_signer_seeds,
            ),
            fee,
        )?;

        Ok(())
    }

    pub fn transfer_nft(&mut self) -> Result<()> {
        let asset_key = self.asset.key();

        let listing_signer_seeds: &[&[&[u8]]] =
            &[&[LISTING, asset_key.as_ref(), &[self.listing.bump]]];

        TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(
                self.collection
                    .as_deref()
                    .map(|collection| collection.as_ref()),
            )
            .payer(&self.maker.to_account_info())
            .authority(Some(&self.listing.to_account_info()))
            .new_owner(&self.buyer.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke_signed(listing_signer_seeds)?;

        Ok(())
    }
}
