use crate::{Offer, OFFER};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};

#[derive(Accounts)]
pub struct CancelTokenOffer<'info> {
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

    pub payment_mint: InterfaceAccount<'info, Mint>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = buyer,
        associated_token::token_program = token_program,
    )]
    pub buyer_payment_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = offer,
        associated_token::token_program = token_program,
    )]
    pub offer_vault_ata: InterfaceAccount<'info, TokenAccount>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> CancelTokenOffer<'info> {
    pub fn refund_tokens(&mut self) -> Result<()> {
        let asset_key = self.asset.key();
        let buyer_key = self.buyer.key();

        let signer_seeds: &[&[&[u8]]] = &[&[
            OFFER,
            asset_key.as_ref(),
            buyer_key.as_ref(),
            &[self.offer.bump],
        ]];

        let amount = self.offer_vault_ata.amount;

        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.offer_vault_ata.to_account_info(),
                    mint: self.payment_mint.to_account_info(),
                    to: self.buyer_payment_ata.to_account_info(),
                    authority: self.offer.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
            self.payment_mint.decimals,
        )?;

        Ok(())
    }

    pub fn close_vault(&mut self) -> Result<()> {
        let asset_key = self.asset.key();
        let buyer_key = self.buyer.key();

        let signer_seeds: &[&[&[u8]]] = &[&[
            OFFER,
            asset_key.as_ref(),
            buyer_key.as_ref(),
            &[self.offer.bump],
        ]];

        close_account(CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            CloseAccount {
                account: self.offer_vault_ata.to_account_info(),
                destination: self.buyer.to_account_info(),
                authority: self.offer.to_account_info(),
            },
            signer_seeds,
        ))?;

        Ok(())
    }
}
