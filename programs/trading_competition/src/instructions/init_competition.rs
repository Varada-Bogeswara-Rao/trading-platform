use anchor_lang::prelude::*;
use crate::competition::{Competition, CompetitionError};
use anchor_lang::solana_program::sysvar::rent::Rent;

#[derive(Accounts)]
#[instruction(duration: i64)]
pub struct InitCompetition<'info> {
    #[account(
        init,
        // WARNING: Using large, manually calculated space like this for Vec<T> is HIGHLY inefficient and risky in Anchor.
        // For a hackathon, we set a large, fixed cap to avoid runtime errors, but ideally use separate PDAs or zero-copy.
        // We calculate a max theoretical size for up to 100 users/positions for now:
        // Discriminator (8) + Pubkey (32) + Pubkey (32) + i64 (8) + i64 (8) + Vec<Pubkey> (4 + 32*100) + Vec<Position> (4 + (8+32+8+8+8+8)*100) + bool (1)
        // Simplified large size allocation to prevent initial failure:
        space = 8 + 16384, // Approx 16KB space (enough for a few positions, but limits future scale)
        payer = authority,
        seeds = [b"competition", authority.key().as_ref()],
        bump
    )]
    pub competition: Account<'info, Competition>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    // Clock is automatically available in Context for handler logic, but not needed in Accounts struct here.
}

pub fn handler(
    ctx: Context<InitCompetition>,
    duration: i64,
    usdc_mint: Pubkey,
) -> Result<()> {
    // Clock is accessed via anchor_lang::prelude::Clock
    let competition = &mut ctx.accounts.competition;
    competition.authority = ctx.accounts.authority.key();
    competition.usdc_mint = usdc_mint;
    
    // NOTE: The current competition model sets the start_time on initialization (L1).
    // This transaction must happen *before* the ER session starts.
    competition.start_time = Clock::get()?.unix_timestamp;
    
    competition.duration = duration;
    competition.users = Vec::new();
    competition.leaderboard = Vec::new();
    competition.is_active = true;
    
    msg!("Competition {} initialized by {}", competition.key(), competition.authority);
    msg!("Starts now: {} for {} seconds", competition.start_time, duration);
    
    Ok(())
}
