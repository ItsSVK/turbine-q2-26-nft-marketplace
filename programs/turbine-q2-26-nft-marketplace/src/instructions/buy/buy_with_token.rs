use crate::{
    error::MarketplaceError, Listing, MarketPlace, MplAsset, MplCollection, MplCore, LISTING,
    MARKETPLACE, MINT_DECIMALS, REWARDS_MINT, TREASURY,
};
use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_2022::{
        mint_to_checked, spl_token_2022::extension::transfer_fee::MAX_FEE_BASIS_POINTS,
        MintToChecked,
    },
    token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked},
};
use mpl_core::instructions::TransferV1CpiBuilder;

#[derive(Accounts)]
pub struct BuyWithToken<'info> {
    #[account(mut)]
    pub taker: Signer<'info>,

    #[account(mut)]
    pub maker: SystemAccount<'info>,

    #[account(
        seeds = [MARKETPLACE, marketplace.name.as_bytes()],
        bump = marketplace.bump,
    )]
    pub marketplace: Box<Account<'info, MarketPlace>>,

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
    pub listing: Box<Account<'info, Listing>>,

    #[account(
        constraint = listing.payment_mint == Some(payment_mint.key())
    )]
    pub payment_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        mint::decimals = MINT_DECIMALS,
        mint::authority = marketplace,
        seeds = [REWARDS_MINT, marketplace.key().as_ref()],
        bump = marketplace.rewards_bump,
    )]
    pub rewards_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        associated_token::mint = payment_mint,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_payment_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = payment_mint,
        associated_token::authority = maker,
        associated_token::token_program = token_program,
    )]
    pub maker_payment_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = payment_mint,
        associated_token::authority = treasury,
        associated_token::token_program = token_program,
    )]
    pub treasury_payment_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = taker,
        associated_token::mint = rewards_mint,
        associated_token::authority = taker,
        associated_token::token_program = token_program,
    )]
    pub taker_reward_ata: InterfaceAccount<'info, TokenAccount>,

    pub system_program: Program<'info, System>,

    pub token_program: Interface<'info, TokenInterface>,

    pub associated_token_program: Program<'info, AssociatedToken>,

    pub mpl_core_program: Program<'info, MplCore>,
}

impl<'info> BuyWithToken<'info> {
    pub fn send_tokens(&mut self) -> Result<()> {
        let price = self.listing.price;

        require!(price > 0, MarketplaceError::InvalidPrice);

        let fee = (price as u128)
            .checked_mul(self.marketplace.fee as u128)
            .ok_or(MarketplaceError::MathOverflow)?
            .checked_div(MAX_FEE_BASIS_POINTS as u128)
            .ok_or(MarketplaceError::MathOverflow)? as u64;

        require!(fee < price, MarketplaceError::InvalidFee);

        let maker_amount = price
            .checked_sub(fee)
            .ok_or(MarketplaceError::MathOverflow)?;

        transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.taker_payment_ata.to_account_info(),
                    mint: self.payment_mint.to_account_info(),
                    to: self.maker_payment_ata.to_account_info(),
                    authority: self.taker.to_account_info(),
                },
            ),
            maker_amount,
            self.payment_mint.decimals,
        )?;

        transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                TransferChecked {
                    from: self.taker_payment_ata.to_account_info(),
                    mint: self.payment_mint.to_account_info(),
                    to: self.treasury_payment_ata.to_account_info(),
                    authority: self.taker.to_account_info(),
                },
            ),
            fee,
            self.payment_mint.decimals,
        )?;

        Ok(())
    }

    pub fn receive_nft(&mut self) -> Result<()> {
        let asset_key = self.asset.key();

        let signer_seeds: &[&[&[u8]]] = &[&[LISTING, asset_key.as_ref(), &[self.listing.bump]]];

        TransferV1CpiBuilder::new(&self.mpl_core_program.to_account_info())
            .asset(&self.asset.to_account_info())
            .collection(
                self.collection
                    .as_deref()
                    .map(|collection| collection.as_ref()),
            )
            .payer(&self.taker.to_account_info())
            .authority(Some(&self.listing.to_account_info()))
            .new_owner(&self.taker.to_account_info())
            .system_program(Some(&self.system_program.to_account_info()))
            .invoke_signed(signer_seeds)?;

        Ok(())
    }

    pub fn receive_rewards(&mut self) -> Result<()> {
        let price = self.listing.price;

        let signer_seeds: &[&[&[u8]]] = &[&[
            MARKETPLACE,
            self.marketplace.name.as_bytes(),
            &[self.marketplace.bump],
        ]];

        mint_to_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                MintToChecked {
                    mint: self.rewards_mint.to_account_info(),
                    to: self.taker_reward_ata.to_account_info(),
                    authority: self.marketplace.to_account_info(),
                },
                signer_seeds,
            ),
            price,
            MINT_DECIMALS,
        )?;

        Ok(())
    }
}
