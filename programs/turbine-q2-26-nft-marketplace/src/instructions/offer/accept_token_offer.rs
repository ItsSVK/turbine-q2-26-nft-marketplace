use crate::{
    error::MarketplaceError, Listing, MarketPlace, MplAsset, MplCollection, MplCore, Offer,
    LISTING, MARKETPLACE, MINT_DECIMALS, OFFER, REWARDS_MINT, TREASURY,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{
        mint_to_checked, spl_token_2022::extension::transfer_fee::MAX_FEE_BASIS_POINTS,
        MintToChecked,
    },
    token_interface::{
        close_account, transfer_checked, CloseAccount, Mint, TokenAccount, TokenInterface,
        TransferChecked,
    },
};
use mpl_core::instructions::TransferV1CpiBuilder;

#[derive(Accounts)]
pub struct AcceptTokenOffer<'info> {
    #[account(mut)]
    pub maker: Signer<'info>,

    #[account(mut)]
    pub buyer: SystemAccount<'info>,

    #[account(
        seeds = [MARKETPLACE, marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Account<'info, MarketPlace>,

    #[account(
        seeds = [TREASURY, marketplace.key().as_ref()],
        bump = marketplace.treasury_bump,
    )]
    pub treasury: SystemAccount<'info>,

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
        constraint = offer.payment_mint == Some(payment_mint.key())
    )]
    pub payment_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = offer,
        associated_token::token_program = token_program,
    )]
    pub offer_vault_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = payment_mint,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_payment_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = payment_mint,
        associated_token::authority = treasury,
        associated_token::token_program = token_program,
    )]
    pub treasury_payment_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        mint::decimals = MINT_DECIMALS,
        mint::authority = marketplace,
        seeds = [REWARDS_MINT, marketplace.key().as_ref()],
        bump = marketplace.rewards_bump,
    )]
    pub rewards_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        init_if_needed,
        payer = maker,
        associated_token::mint = rewards_mint,
        associated_token::authority = buyer,
        associated_token::token_program = token_program,
    )]
    pub buyer_reward_ata: Box<InterfaceAccount<'info, TokenAccount>>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub system_program: Program<'info, System>,

    pub mpl_core_program: Program<'info, MplCore>,
}

impl<'info> AcceptTokenOffer<'info> {
    pub fn send_tokens(&mut self) -> Result<()> {
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

        let offer_signer_seeds: &[&[&[u8]]] = &[&[
            OFFER,
            asset_key.as_ref(),
            buyer_key.as_ref(),
            &[self.offer.bump],
        ]];

        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.offer_vault_ata.to_account_info(),
                    mint: self.payment_mint.to_account_info(),
                    to: self.maker_payment_ata.to_account_info(),
                    authority: self.offer.to_account_info(),
                },
                offer_signer_seeds,
            ),
            maker_amount,
            self.payment_mint.decimals,
        )?;

        transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.offer_vault_ata.to_account_info(),
                    mint: self.payment_mint.to_account_info(),
                    to: self.treasury_payment_ata.to_account_info(),
                    authority: self.offer.to_account_info(),
                },
                offer_signer_seeds,
            ),
            fee,
            self.payment_mint.decimals,
        )?;

        Ok(())
    }

    pub fn close_vault(&mut self) -> Result<()> {
        let asset_key = self.asset.key();
        let buyer_key = self.buyer.key();

        let offer_signer_seeds: &[&[&[u8]]] = &[&[
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
            offer_signer_seeds,
        ))?;

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

    pub fn mint_rewards(&mut self) -> Result<()> {
        let amount = self.offer.amount;

        let marketplace_signer_seeds: &[&[&[u8]]] = &[&[
            MARKETPLACE,
            self.marketplace.name.as_bytes(),
            &[self.marketplace.bump],
        ]];

        mint_to_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintToChecked {
                    mint: self.rewards_mint.to_account_info(),
                    to: self.buyer_reward_ata.to_account_info(),
                    authority: self.marketplace.to_account_info(),
                },
                marketplace_signer_seeds,
            ),
            amount,
            MINT_DECIMALS,
        )?;

        Ok(())
    }
}
