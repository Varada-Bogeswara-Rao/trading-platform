use anchor_lang::prelude::*;
use crate::competition::*;
use hex;

#[derive(Accounts)]
pub struct FinalCommit<'info> {
    #[account(
        mut,
        has_one = authority,
        has_one = er_instance,
        constraint = competition.phase == CompetitionPhase::Active @ CompetitionError::NotActive
    )]
    pub competition: Account<'info, Competition>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: Must be competition.er_instance
    #[account(address = competition.er_instance @ CompetitionError::Unauthorized)]
    pub er_instance: UncheckedAccount<'info>,

    pub clock: Sysvar<'info, Clock>,
}

pub fn handler(
    ctx: Context<FinalCommit>,
    er_instance: Pubkey,
    state_root: [u8; 32],
    winner_pubkey: Pubkey,
    winner_profit: i128,
) -> Result<()> {
    let comp = &mut ctx.accounts.competition;
    let now = ctx.accounts.clock.unix_timestamp;

    // sanity checks
    require!(er_instance == comp.er_instance, CompetitionError::Unauthorized);
    require!(now >= comp.end_time, CompetitionError::NotEnded);

    comp.state_root = state_root;
    comp.winner = winner_pubkey;
    comp.winner_profit = winner_profit;
    comp.phase = CompetitionPhase::Finalizing;
    comp.challenge_deadline = now + 300; // 5 min window

    msg!(
        "Final state root {} committed – challenge until {}",
        hex::encode(state_root),
        comp.challenge_deadline
    );
    Ok(())
}