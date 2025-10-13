use anchor_lang::prelude::*;
use crate::competition::{Competition, CompetitionError};
use anchor_lang::solana_program::sysvar::rent::Rent;
use magicblock_sdk::{DelegateAccount, delegation_program}; // Import the delegation_program module

#[derive(Accounts)]
pub struct DelegateAccounts<'info> {
    #[account(mut, has_one = authority)]
    pub competition: Account<'info, Competition>,
    #[account(mut)]
    pub user: Signer<'info>,
    // --- The Position PDA will be delegated to the ER ---
    #[account(
        init,
        payer = user,
        // The space calculation for Position needs to be accurate
        space = 8 + 32 + 8 + 8 + 8 + 8, // Discriminator + Pubkey + 4x u64/i64 = 8 + 64 bytes
        seeds = [b"position", competition.key().as_ref(), user.key().as_ref()],
        bump
    )]
    pub position: Account<'info, crate::competition::Position>,
    // --- MagicBlock CPI Accounts ---
    /// CHECK: MagicBlock Delegation Program account
    #[account(address = delegation_program::ID)]
    pub delegation_program: UncheckedAccount<'info>,
    /// CHECK: This is the specific Ephemeral Rollup instance address provided by the user (or client)
    pub er_instance: UncheckedAccount<'info>, 
    // ---------------------------------
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<DelegateAccounts>,
    er_instance: Pubkey,
) -> Result<()> {
    let competition = &mut ctx.accounts.competition;
    require!(competition.is_active, CompetitionError::NotActive);

    let user_key = ctx.accounts.user.key();
    
    // 1. Initialize user position (L1 transaction)
    let position = &mut ctx.accounts.position;
    position.user = user_key;
    position.usdc_balance = 1000_000_000; // Initial synthetic capital
    position.initial_value = 1000_000_000;
    position.current_value = 1000_000_000;
    position.profit = 0;

    // Add user to competition (if not already there)
    // NOTE: This logic needs protection against the competition account hitting max size.
    // For the hackathon, we assume a small number of users.
    if !competition.users.contains(&user_key) {
        competition.users.push(user_key);
    }
    
    // 2. Delegate the Position account to the ER (L1 transaction with CPI)
    let cpi_program = ctx.accounts.delegation_program.to_account_info();
    
    // We need the signer seeds for the Position PDA since it's being delegated by the Competition Program
    let position_seeds = &[
        b"position",
        competition.to_account_info().key.as_ref(),
        user_key.as_ref(),
        &[ctx.bumps.get("position").unwrap()],
    ];

    let delegate_instruction = DelegateAccount {
        // Accounts to be delegated. Only the Position PDA needs high-speed writes.
        accounts_to_delegate: vec![
            ctx.accounts.position.to_account_info(),
        ],
        er_instance,
        payer: ctx.accounts.user.to_account_info().key(),
    };

    // Use the delegation program's CPI function
    magicblock_sdk::cpi::delegate_account(
        CpiContext::new_with_signer(
            cpi_program,
            delegate_instruction.to_account_metas(None),
            &[position_seeds],
        ),
        delegate_instruction,
    )?;

    msg!("User {} position delegated to ER instance {}", user_key, er_instance);

    Ok(())
}
