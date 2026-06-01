use crate::error::MarketplaceError;
use crate::{MarketPlace, MARKETPLACE, TREASURY};
use anchor_lang::prelude::*;
use anchor_lang::system_program::{transfer, Transfer};

#[derive(Accounts)]
pub struct ClaimSolFee<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [MARKETPLACE, marketplace.name.as_bytes()],
        bump = marketplace.bump,
        has_one = admin,
    )]
    pub marketplace: Account<'info, MarketPlace>,

    #[account(
        mut,
        seeds = [TREASURY, marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,

    #[account(mut)]
    pub recipient: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

impl<'info> ClaimSolFee<'info> {
    pub fn claim_fees(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, MarketplaceError::InvalidFee);
        let treasury_info = self.treasury.to_account_info();
        let treasury_balance = treasury_info.lamports();

        let rent = Rent::get()?;
        let min_rent = rent.minimum_balance(treasury_info.data_len());

        require!(
            treasury_balance.saturating_sub(amount) >= min_rent,
            MarketplaceError::InsufficientTreasuryFunds
        );

        let marketplace_key = self.marketplace.key();

        let signer_seeds: &[&[&[u8]]] = &[&[
            TREASURY,
            marketplace_key.as_ref(),
            &[self.marketplace.treasury_bump],
        ]];

        transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                Transfer {
                    from: treasury_info,
                    to: self.recipient.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;

        Ok(())
    }
}
