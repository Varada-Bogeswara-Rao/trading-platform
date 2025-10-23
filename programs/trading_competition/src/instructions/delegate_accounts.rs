use anchor_lang::prelude::*;
use crate::competition::*;
use anchor_spl::token::{Token, TokenAccount};

use ephemeral_rollups_sdk::cpi::*;
use ephemeral_rollups_sdk::consts::DELEGATION_PROGRAM_ID;
// use ephemeral_rollups_sdk::delegate_args::*;

#[derive(Accounts)]
pub struct DelegateAccounts<'info> {
    #[account(
        mut,
        has_one = authority,
        has_one = er_instance,
        constraint = competition.phase == CompetitionPhase::Upcoming @ CompetitionError::NotActive
    )]
    pub competition: Account<'info, Competition>,

    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        token::mint = competition.usdc_mint,
        token::authority = user
    )]
    pub user_usdc_ata: Account<'info, TokenAccount>,

    #[account(
        init,
        payer = user,
        space = 8 + 32*3 + 16*3 + 1,
        seeds = [b"position", competition.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub position: Account<'info, Position>,

    /// CHECK: MagicBlock delegation program
    #[account(address = DELEGATION_PROGRAM_ID)]
    pub delegation_program: UncheckedAccount<'info>,

    /// CHECK: Must equal competition.er_instance
    #[account(address = competition.er_instance)]
    pub er_instance: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(ctx: Context<DelegateAccounts>, _er_instance: Pubkey) -> Result<()> {
    let comp = &ctx.accounts.competition;
    let pos = &mut ctx.accounts.position;

    // initialise synthetic balance (demo: 1 M USDC)
    pos.competition = comp.key();
    pos.user = ctx.accounts.user.key();
    pos.usdc_ata = ctx.accounts.user_usdc_ata.key();
    pos.usdc_balance = 1_000_000_000_000u128; // 1 M * 10^6
    pos.initial_value = pos.usdc_balance;
    pos.current_value = pos.usdc_balance;
    pos.bump = ctx.bumps.position;

    // CPI to delegate
    let cpi_prog = ctx.accounts.delegation_program.to_account_info();
    let cpi_accounts = DelegateAccount {
        accounts_to_delegate: vec![
            ctx.accounts.position.to_account_info(),
            ctx.accounts.user_usdc_ata.to_account_info(),
        ],
        er_instance: ctx.accounts.er_instance.to_account_info(),
        payer: ctx.accounts.user.to_account_info(),
    };
    let data = DelegateInstructionData {
        er_instance: comp.er_instance,
        payer: ctx.accounts.user.key(),
        accounts_to_delegate: vec![pos.key(), ctx.accounts.user_usdc_ata.key()],
    };

    delegate_account(CpiContext::new(cpi_prog, cpi_accounts), data)?;

    msg!("User {} delegated to ER {}", ctx.accounts.user.key(), comp.er_instance);
    Ok(())
}