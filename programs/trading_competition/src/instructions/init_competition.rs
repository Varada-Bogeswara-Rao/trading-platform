use anchor_lang::prelude::*;

use crate::competition::*;

#[derive(Accounts)]
pub struct InitCompetition<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Competition::INIT_SPACE,
        seeds = [b"competition", authority.key().as_ref()],
        bump
    )]
    pub competition: Account<'info, Competition>,

    #[account(
        init,
        payer = authority,
        space = 8 + MockPriceAccount::INIT_SPACE,
        seeds = [b"mock_price", competition.key().as_ref()],
        bump,
    )]
    pub mock_price: Account<'info, MockPriceAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub usdc_mint: Account<'info, anchor_spl::token::Mint>,

    /// CHECK: Verified in handler
    pub er_instance: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}
pub fn handler(
    ctx: Context<InitCompetition>,
    duration: i64,
    usdc_mint: Pubkey,
    er_instance: Pubkey,
) -> Result<()> {
    let competition = &mut ctx.accounts.competition;
    let clock = Clock::get()?;

    // Immutable borrow first
    let mock_price_key = ctx.accounts.mock_price.key();
    let mock_price = &mut ctx.accounts.mock_price;

    competition.authority = ctx.accounts.authority.key();
    competition.usdc_mint = usdc_mint;
    competition.er_instance = er_instance;
    competition.mock_price_pda = mock_price_key;
    competition.phase = CompetitionPhase::Active;
    competition.start_time = clock.unix_timestamp;
    competition.end_time = competition.start_time + duration;
    competition.challenge_deadline = competition.end_time + 3600;
    competition.winner = Pubkey::default();
    competition.winner_profit = 0;
    competition.state_root = [0u8; 32];
    competition.bump = ctx.bumps.competition;

    // Mutable borrow: update price
    mock_price.update_price(150_000_000u128, -8i32, &clock)?;

    Ok(())
}
