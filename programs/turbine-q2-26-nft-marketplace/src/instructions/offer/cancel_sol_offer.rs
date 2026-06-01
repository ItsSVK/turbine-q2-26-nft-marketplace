use crate::{Offer, OFFER, OFFER_VAULT};
use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};

#[derive(Accounts)]
pub struct CancelSolOffer<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    /// CHECK: asset key only used as a seed
    pub asset: UncheckedAccount<'info>,

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

    pub system_program: Program<'info, System>,
}

impl<'info> CancelSolOffer<'info> {
    pub fn refund_sol(&mut self) -> Result<()> {
        let asset_key = self.asset.key();
        let buyer_key = self.buyer.key();
        let vault_bump = [self.offer.vault_bump.unwrap()];

        let vault_signer_seeds: &[&[&[u8]]] = &[&[
            OFFER_VAULT,
            asset_key.as_ref(),
            buyer_key.as_ref(),
            &vault_bump,
        ]];

        let amount = self.offer_vault.lamports();

        transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.offer_vault.to_account_info(),
                    to: self.buyer.to_account_info(),
                },
                vault_signer_seeds,
            ),
            amount,
        )?;

        Ok(())
    }
}
