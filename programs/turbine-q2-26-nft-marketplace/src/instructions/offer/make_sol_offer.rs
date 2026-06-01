use crate::{error::MarketplaceError, Offer, OFFER, OFFER_VAULT};
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

#[derive(Accounts)]
pub struct MakeSolOffer<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    /// CHECK: asset key only used as a seed
    pub asset: AccountInfo<'info>,

    #[account(
        init,
        payer = buyer,
        space = Offer::DISCRIMINATOR.len() + Offer::INIT_SPACE,
        seeds = [OFFER, asset.key().as_ref(), buyer.key().as_ref()],
        bump
    )]
    pub offer: Account<'info, Offer>,

    #[account(
        mut,
        seeds = [OFFER_VAULT, asset.key().as_ref(), buyer.key().as_ref()],
        bump,
    )]
    pub offer_vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> MakeSolOffer<'info> {
    pub fn init_offer(&mut self, amount: u64, bumps: &MakeSolOfferBumps) -> Result<()> {
        require!(amount > 0, MarketplaceError::InvalidPrice);

        self.offer.set_inner(Offer {
            buyer: self.buyer.key(),
            asset: self.asset.key(),
            amount,
            bump: bumps.offer,
            vault_bump: Some(bumps.offer_vault),
            payment_mint: None,
        });

        Ok(())
    }

    pub fn deposit_sol(&mut self, amount: u64) -> Result<()> {
        transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.buyer.to_account_info(),
                    to: self.offer_vault.to_account_info(),
                },
            ),
            amount,
        )?;

        Ok(())
    }
}
