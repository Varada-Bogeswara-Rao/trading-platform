use anchor_lang::prelude::*;
use crate::competition::*;
use crate::events::UserUndelegated;
use anchor_spl::token::TokenAccount;
// use ephemeral_rollups_sdk::cpi::*;
use ephemeral_rollups_sdk::consts::DELEGATION_PROGRAM_ID;
// use ephemeral_rollups_sdk::delegate_args::*;

#[derive(Accounts)]
#[instruction(er_instance_key: Pubkey)]
pub struct UserUndelegate<'info> {
    #[account(
        has_one = usdc_mint,
        has_one = er_instance,
        constraint = matches!(competition.phase, CompetitionPhase::Finalizing | CompetitionPhase::Settled)
            @ CompetitionError::NotActive
    )]
    pub competition: Account<'info, Competition>,

    #[account(
        mut,
        has_one = user,
        has_one = usdc_ata,
        seeds = [b"position", competition.key().as_ref(), user.key().as_ref()],
        bump = position.bump
    )]
    pub position: Account<'info, Position>,

    #[account(mut)]
    pub usdc_ata: Account<'info, TokenAccount>,

    pub user: Signer<'info>,

    /// CHECK: MagicBlock delegation program
    #[account(address = DELEGATION_PROGRAM_ID)]
    pub delegation_program: UncheckedAccount<'info>,

    /// CHECK: Must match competition.er_instance
    #[account(address = competition.er_instance @ CompetitionError::Unauthorized)]
    pub er_instance: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handler(ctx: Context<UserUndelegate>, er_instance_key: Pubkey) -> Result<()> {
    let comp = &ctx.accounts.competition;
    let pos = &ctx.accounts.position;

    require!(er_instance_key == comp.er_instance, CompetitionError::Unauthorized);
    require!(ctx.accounts.er_instance.key() == er_instance_key, CompetitionError::Unauthorized);

    let profit = pos.profit();

    // CPI – commit Position state, then undelegate both accounts
    let cpi_prog = ctx.accounts.delegation_program.to_account_info();
    let cpi_accounts = CommitAndUndelegateAccounts {
        payer: ctx.accounts.user.to_account_info(),
        er_instance: ctx.accounts.er_instance.to_account_info(),
        accounts_to_settle: vec![
            ctx.accounts.position.to_account_info(),
            ctx.accounts.usdc_ata.to_account_info(),
        ],
    };
    let data = CommitUndelegateInstructionData {
        er_instance: er_instance_key,
        payer: ctx.accounts.user.key(),
        accounts_to_commit: vec![pos.key()],          // only PDA needs commit
        accounts_to_undelegate: vec![pos.key(), ctx.accounts.usdc_ata.key()],
    };

    commit_and_undelegate_accounts(CpiContext::new(cpi_prog, cpi_accounts), data)?;

    emit!(UserUndelegated {
        user: pos.user,
        competition: comp.key(),
        final_pnl: profit,
        timestamp: Clock::get()?.unix_timestamp,
    });

    msg!("User {} undelegated – final PnL ${}", pos.user, profit);
    Ok(())
}