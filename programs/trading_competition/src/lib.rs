use anchor_lang::prelude::*;

pub mod competition;
pub mod events;
pub mod instructions;

use instructions::*;

declare_id!("HjmkkHv5A1SPbL4zjpRJjYVj33YTTq9QYyCPkx6x6HnB");

#[program]
pub mod trading_competition {
    use super::*;

    pub fn init_competition(
        ctx: Context<InitCompetition>,
        duration: i64,
        usdc_mint: Pubkey,
        er_instance: Pubkey,
    ) -> Result<()> {
        instructions::init_competition::handler(ctx, duration, usdc_mint, er_instance)
    }

    pub fn delegate_accounts(ctx: Context<DelegateAccounts>) -> Result<()> {
        instructions::delegate_accounts::handler(ctx, ctx.accounts.competition.er_instance)
    }

    pub fn process_trade(
        ctx: Context<ProcessTrade>,
        amount: u64,
        is_buy: bool,
        price_feed_id: Pubkey,
    ) -> Result<()> {
        instructions::process_trade::handler(ctx, amount, is_buy)
    }

    pub fn final_commit(
        ctx: Context<FinalCommit>,
        er_instance: Pubkey,
        state_root: [u8; 32],
        winner_pubkey: Pubkey,
        winner_profit: i128,
    ) -> Result<()> {
        instructions::final_commit::handler(
            ctx,
            er_instance,
            state_root,
            winner_pubkey,
            winner_profit,
        )
    }

    pub fn user_undelegate(
        ctx: Context<UserUndelegate>,
        er_instance_key: Pubkey,
    ) -> Result<()> {
        instructions::user_undelegate::handler(ctx, er_instance_key)
    }

    pub fn settle_competition(ctx: Context<SettleCompetition>) -> Result<()> {
        instructions::settle_competition::handler(ctx)
    }

    pub fn mint_winner_nft(
        ctx: Context<MintWinnerNft>,
        metadata_uri: String,
    ) -> Result<()> {
        instructions::mint_winner_nft::handler(ctx, metadata_uri)
    }
}