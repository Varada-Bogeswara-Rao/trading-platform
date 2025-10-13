use anchor_lang::prelude::*;
use crate::competition::{Competition, CompetitionError};
use anchor_lang::solana_program::sysvar::clock::Clock;
use magicblock_sdk::{ephemeral_rollups::CommitAndUndelegateAccounts, delegation_program}; // Updated import for proper CPI

#[derive(Accounts)]
pub struct FinalCommit<'info> {
    #[account(mut, has_one = authority)]
    pub competition: Account<'info, Competition>,
    pub authority: Signer<'info>,
    /// CHECK: MagicBlock Delegation Program account
    #[account(address = delegation_program::ID)]
    pub delegation_program: UncheckedAccount<'info>,
    /// CHECK: The specific Ephemeral Rollup instance that ran the competition
    pub er_instance: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<FinalCommit>,
    er_instance: Pubkey,
) -> Result<()> {
    let competition = &mut ctx.accounts.competition;
    require!(competition.is_active, CompetitionError::NotActive);

    // 1. Time Check: Ensure the competition duration has actually ended.
    let current_time = Clock::get()?.unix_timestamp;
    let end_time = competition.start_time.checked_add(competition.duration).unwrap();
    require!(
        current_time >= end_time,
        CompetitionError::NotEnded
    );

    // 2. Identify all delegated accounts for settlement.
    // NOTE: This array needs to include the Competition account itself if it was delegated, 
    // plus ALL individual user position accounts that were delegated.
    // For simplicity, we are assuming only the Competition account needs final commitment 
    // (since it holds the final leaderboard state). 
    // In a real app, you would require the Position PDAs here too.
    let accounts_to_settle = vec![
        ctx.accounts.competition.to_account_info(),
        // Add all user Position PDAs here dynamically if they were individually delegated
    ];

    // 3. Commit state and Undelegate (L1 CPI)
    // We use the combined CommitAndUndelegate instruction which is the correct pattern.
    // The authority (signer) pays for and executes this final settlement transaction.
    
    // CPI Context setup
    let cpi_program = ctx.accounts.delegation_program.to_account_info();
    
    // Since the competition account is a PDA, we need its seeds for the CPI.
    // NOTE: We assume the Competition account PDA seeds here.
    let competition_seeds = &[
        b"competition",
        ctx.accounts.authority.key().as_ref(),
        &[ctx.bumps.get("competition").unwrap()], 
    ];

    magicblock_sdk::cpi::commit_and_undelegate_accounts(
        CpiContext::new_with_signer(
            cpi_program,
            CommitAndUndelegateAccounts {
                payer: ctx.accounts.authority.to_account_info(),
                er_instance: ctx.accounts.er_instance.to_account_info(),
                accounts_to_settle,
            },
            &[competition_seeds]
        ),
    )?;

    // 4. Update status after successful commitment
    competition.is_active = false;
    msg!("Competition finalized. State committed and undelegated to Solana L1.");
    
    Ok(())
}
