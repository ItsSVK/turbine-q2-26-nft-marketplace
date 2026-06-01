pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("GdpmNmSGvz9AkftRpjdCXKeRroQie68vzUufvZTyCy8V");

#[program]
pub mod turbine_q2_26_nft_marketplace {
    use super::*;

    pub fn init(ctx: Context<Init>, name: String, fee: u16) -> Result<()> {
        ctx.accounts.init(name, fee, &ctx.bumps)
    }

    pub fn list(ctx: Context<List>, price: u64) -> Result<()> {
        ctx.accounts.create_listing(price, &ctx.bumps)
    }

    pub fn delist(ctx: Context<Delist>) -> Result<()> {
        ctx.accounts.cancel_listing()
    }

    pub fn buy_with_sol(ctx: Context<BuyWithSol>) -> Result<()> {
        ctx.accounts.send_sol()?;
        ctx.accounts.receive_nft()?;
        ctx.accounts.receive_rewards()
    }

    pub fn buy_with_token(ctx: Context<BuyWithToken>) -> Result<()> {
        ctx.accounts.send_tokens()?;
        ctx.accounts.receive_nft()?;
        ctx.accounts.receive_rewards()
    }

    pub fn claim_sol_fee(ctx: Context<ClaimSolFee>, amount: u64) -> Result<()> {
        ctx.accounts.claim_fees(amount)
    }

    pub fn claim_token_fee(ctx: Context<ClaimTokenFee>, amount: u64) -> Result<()> {
        ctx.accounts.claim_fees(amount)
    }

    pub fn make_sol_offer(ctx: Context<MakeSolOffer>, amount: u64) -> Result<()> {
        ctx.accounts.init_offer(amount, &ctx.bumps)?;
        ctx.accounts.deposit_sol(amount)
    }

    pub fn accept_sol_offer(ctx: Context<AcceptSolOffer>) -> Result<()> {
        ctx.accounts.send_sol()?;
        ctx.accounts.transfer_nft()
    }

    pub fn cancel_sol_offer(ctx: Context<CancelSolOffer>) -> Result<()> {
        ctx.accounts.refund_sol()
    }

    pub fn make_token_offer(ctx: Context<MakeTokenOffer>, amount: u64) -> Result<()> {
        ctx.accounts.init_offer(amount, &ctx.bumps)?;
        ctx.accounts.deposit_tokens(amount)
    }

    pub fn accept_token_offer(ctx: Context<AcceptTokenOffer>) -> Result<()> {
        ctx.accounts.send_tokens()?;
        ctx.accounts.close_vault()?;
        ctx.accounts.transfer_nft()?;
        ctx.accounts.mint_rewards()
    }

    pub fn cancel_token_offer(ctx: Context<CancelTokenOffer>) -> Result<()> {
        ctx.accounts.refund_tokens()?;
        ctx.accounts.close_vault()
    }
}
