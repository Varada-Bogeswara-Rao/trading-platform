use anchor_lang::prelude::*;

use crate::competition::{Competition, MockPriceAccount, CompetitionError, CompetitionPhase};  // Fixed: Import Phase


#[derive(Accounts)]
pub struct UpdateMockPrice<'info> {
    #[account(
        mut,
        has_one = authority,
        seeds = [b"competition", competition.key().as_ref()],
        bump = competition.bump,
    )]
    pub competition: Account<'info, Competition>,

    #[account(
        mut,
        seeds = [b"mock_price", competition.key().as_ref()],
        bump = mock_price.bump,
    )]
    pub mock_price: Account<'info, MockPriceAccount>,

    pub authority: Signer<'info>,
}

pub fn handler(ctx: Context<UpdateMockPrice>, new_price: u128, new_expo: i32) -> Result<()> {
    let clock = Clock::get()?;
    let now = clock.unix_timestamp;

    require!(ctx.accounts.competition.phase == CompetitionPhase::Active, CompetitionError::NotActive);
    require!(now < ctx.accounts.competition.end_time, CompetitionError::NotEnded);

    ctx.accounts.mock_price.update_price(new_price, new_expo, &clock)?;

    Ok(())
}