use anchor_lang::prelude::*;
use crate::competition::*;

#[derive(Accounts)]
pub struct SettleCompetition<'info> {
    #[account(
        mut,
        has_one = authority,
        constraint = competition.phase == CompetitionPhase::Finalizing @ CompetitionError::NotActive
    )]
    pub competition: Account<'info, Competition>,
    pub authority: Signer<'info>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn handler(ctx: Context<SettleCompetition>) -> Result<()> {
    let comp = &mut ctx.accounts.competition;
    require!(
        Clock::get()?.unix_timestamp >= comp.challenge_deadline,
        CompetitionError::NotEnded
    );
    comp.phase = CompetitionPhase::Settled;
    msg!("Competition {} settled – NFT minting now safe", comp.key());
    Ok(())
}