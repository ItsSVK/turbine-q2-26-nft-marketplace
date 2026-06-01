use crate::{error::MarketplaceError, Offer, OFFER};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};

#[derive(Accounts)]
pub struct MakeTokenOffer<'info> {
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

    pub payment_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = buyer,
        associated_token::token_program = token_program,
    )]
    pub buyer_payment_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = payment_mint,
        associated_token::authority = offer,
        associated_token::token_program = token_program,
    )]
    pub offer_vault_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,
}

impl<'info> MakeTokenOffer<'info> {
    pub fn init_offer(&mut self, amount: u64, bumps: &MakeTokenOfferBumps) -> Result<()> {
        require!(amount > 0, MarketplaceError::InvalidPrice);

        self.offer.set_inner(Offer {
            buyer: self.buyer.key(),
            asset: self.asset.key(),
            amount,
            bump: bumps.offer,
            vault_bump: None,
            payment_mint: Some(self.payment_mint.key()),
        });

        Ok(())
    }

    pub fn deposit_tokens(&mut self, amount: u64) -> Result<()> {
        transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.buyer_payment_ata.to_account_info(),
                    mint: self.payment_mint.to_account_info(),
                    to: self.offer_vault_ata.to_account_info(),
                    authority: self.buyer.to_account_info(),
                },
            ),
            amount,
            self.payment_mint.decimals,
        )?;

        Ok(())
    }
}
