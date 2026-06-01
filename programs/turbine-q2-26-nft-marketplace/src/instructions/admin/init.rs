use crate::error::MarketplaceError;
use crate::state::MarketPlace;
use crate::{MARKETPLACE, MAX_NAME_LENGTH, MINT_DECIMALS, REWARDS_MINT, TREASURY};
use anchor_lang::prelude::*;
use anchor_spl::token_2022::spl_token_2022::extension::transfer_fee::MAX_FEE_BASIS_POINTS;
use anchor_spl::token_interface::{Mint, TokenInterface};

#[derive(Accounts)]
#[instruction(name: String)]
pub struct Init<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init,
        payer = admin,
        space = MarketPlace::DISCRIMINATOR.len() + MarketPlace::INIT_SPACE,
        seeds = [MARKETPLACE, name.as_str().as_bytes()],
        bump,
    )]
    pub marketplace: Box<Account<'info, MarketPlace>>,

    #[account(
        seeds = [TREASURY, marketplace.key().as_ref()],
        bump,
    )]
    pub treasury: SystemAccount<'info>,

    #[account(
        init,
        payer = admin,
        mint::decimals = MINT_DECIMALS,
        mint::authority = marketplace,
        seeds = [REWARDS_MINT, marketplace.key().as_ref()],
        bump
    )]
    pub rewards_mint: InterfaceAccount<'info, Mint>,

    pub system_program: Program<'info, System>,

    pub token_program: Interface<'info, TokenInterface>,
}

impl<'info> Init<'info> {
    pub fn init(&mut self, name: String, fee: u16, bumps: &InitBumps) -> Result<()> {
        require!(!name.trim().is_empty(), MarketplaceError::EmptyName);

        require!(
            name.len() <= MAX_NAME_LENGTH as usize,
            MarketplaceError::NameTooLong
        );

        require!(fee < MAX_FEE_BASIS_POINTS, MarketplaceError::InvalidFee);

        self.marketplace.set_inner(MarketPlace {
            admin: self.admin.key(),
            bump: bumps.marketplace,
            fee,
            name,
            rewards_bump: bumps.rewards_mint,
            treasury_bump: bumps.treasury,
        });

        Ok(())
    }
}
