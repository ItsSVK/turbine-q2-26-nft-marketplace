use crate::error::MarketplaceError;
use crate::{MarketPlace, MARKETPLACE, TREASURY};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

#[derive(Accounts)]
pub struct ClaimTokenFee<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [MARKETPLACE, marketplace.name.as_bytes()],
        bump = marketplace.bump,
        has_one = admin,
    )]
    pub marketplace: Account<'info, MarketPlace>,

    #[account(
        seeds = [TREASURY, marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = treasury,
        associated_token::token_program = token_program,
    )]
    pub treasury_payment_ata: InterfaceAccount<'info, TokenAccount>,

    pub payment_mint: InterfaceAccount<'info, Mint>,

    pub recipient: SystemAccount<'info>,

    #[account(
        init_if_needed,
        payer = admin,
        associated_token::mint = payment_mint,
        associated_token::authority = recipient,
        associated_token::token_program = token_program,
    )]
    pub recipient_payment_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,
}

impl<'info> ClaimTokenFee<'info> {
    pub fn claim_fees(&mut self, amount: u64) -> Result<()> {
        require!(amount > 0, MarketplaceError::InvalidFee);

        let treasury_balance = self.treasury_payment_ata.amount;

        require!(
            treasury_balance >= amount,
            MarketplaceError::InsufficientTreasuryFunds
        );

        let marketplace_key = self.marketplace.key();

        let signer_seeds: &[&[&[u8]]] = &[&[
            TREASURY,
            marketplace_key.as_ref(),
            &[self.marketplace.treasury_bump],
        ]];

        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.treasury_payment_ata.to_account_info(),
                    mint: self.payment_mint.to_account_info(),
                    to: self.recipient_payment_ata.to_account_info(),
                    authority: self.treasury.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
            self.payment_mint.decimals,
        )?;

        Ok(())
    }
}
