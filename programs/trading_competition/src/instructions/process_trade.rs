use anchor_lang::prelude::*;
use crate::competition::{Competition, Position, CompetitionError};
// NOTE: Pyth Lazer usage is typically done via CPI to the Pyth Lazer program, 
// or by reading a dedicated price feed account, not directly via `PriceFeed::fetch`
// if Pyth Lazer is set up as a custom MagicBlock plugin.
// For this program, we simulate the logic assuming the price is correctly obtained.
// We keep the Pyth SDK import for now to indicate intent.
use pyth_lazer_sdk::{PriceFeed, PriceFeedId}; 
use anchor_lang::solana_program::sysvar::clock::Clock; // Use Clock for time checks

#[derive(Accounts)]
pub struct ProcessTrade<'info> {
    // The Competition account tracks metadata and the full leaderboard
    #[account(mut)]
    pub competition: Account<'info, Competition>,
    // The Position account tracks the user's personal state (delegated to ER)
    #[account(mut, has_one = user)]
    pub position: Account<'info, Position>,
    pub user: Signer<'info>,
    // Pyth Lazer Price Feed Account (read-only)
    /// CHECK: This account holds the real-time price data from Pyth Lazer/MagicBlock's Oracle
    pub price_feed_account: UncheckedAccount<'info>, 
}

pub fn handler(
    ctx: Context<ProcessTrade>,
    amount: u64,
    is_buy: bool,
    _price_feed_id: Pubkey, // We use the account key directly
) -> Result<()> {
    let competition = &mut ctx.accounts.competition;
    
    // 1. Time & Authorization Checks (CRITICAL)
    require!(competition.is_active, CompetitionError::NotActive);
    
    let now = Clock::get()?.unix_timestamp;
    let end_time = competition.start_time.checked_add(competition.duration).unwrap();
    require!(now < end_time, CompetitionError::NotEnded);
    
    require!(
        competition.users.contains(&ctx.accounts.user.key()),
        CompetitionError::Unauthorized
    );

    // 2. Fetch Price (Simulated)
    // NOTE: In a real environment, you would use Pyth Lazer SDK to safely read the price 
    // from the `price_feed_account` passed in the context. We simulate the data extraction
    // here, assuming the price feed account data is correctly structured and verified.
    // For simplicity, we use a placeholder `price` value derived from the price feed data.
    let price_feed_data = &ctx.accounts.price_feed_account.try_borrow_data()?;
    // **Simulated Price Extraction (Actual Pyth Lazer logic required here)**
    let price: u64 = if price_feed_data.len() > 16 {
        // Placeholder: use 100000 as a mock price ($1.00000, assuming 5 decimal places)
        100000 
    } else {
        return err!(CompetitionError::InvalidPriceFeed);
    };

    // 3. Trade Execution (PnL Calculation)
    let position = &mut ctx.accounts.position;
    // Assume price has 5 decimals for this example (Pyth Lazer prices vary)
    const PRICE_DECIMALS: u64 = 100000; 

    // Calculate USD value of the trade (amount * price)
    let trade_value = amount.checked_mul(price).unwrap() / PRICE_DECIMALS;
    
    if is_buy {
        // User spends USDC, portfolio value increases
        position.usdc_balance = position.usdc_balance.checked_sub(trade_value).unwrap();
        position.current_value = position.current_value.checked_add(trade_value).unwrap();
    } else {
        // User receives USDC, portfolio value decreases
        position.usdc_balance = position.usdc_balance.checked_add(trade_value).unwrap();
        position.current_value = position.current_value.checked_sub(trade_value).unwrap();
    }

    // 4. Update Profit
    position.profit = (position.current_value as i64)
        .checked_sub(position.initial_value as i64)
        .unwrap();
    
    msg!("User {} traded. New Balance: {}. Current PnL: {}", 
        position.user, position.usdc_balance, position.profit);

    // 5. Update Leaderboard (CRITICAL STEP FOR ER STATE)
    // Since the Competition account is delegated to the ER, this update is fast.
    update_leaderboard(competition, position)?;

    Ok(())
}

// Helper function for Leaderboard update
fn update_leaderboard(competition: &mut Competition, position: &Position) -> Result<()> {
    // Find and update the user's position in the leaderboard vector
    let user_position = competition
        .leaderboard
        .iter_mut()
        .find(|p| p.user == position.user);
    
    match user_position {
        Some(p) => {
            p.current_value = position.current_value;
            p.profit = position.profit;
            p.usdc_balance = position.usdc_balance; // Update balance as well
        }
        None => {
            // User might be added in delegate_accounts, but if not, add here.
            competition.leaderboard.push(position.clone());
        }
    }

    // Sort the leaderboard by profit (highest first)
    // NOTE: Sorting a Vec on-chain is computationally expensive, but acceptable 
    // for a hackathon demo with limited users.
    competition.leaderboard.sort_by(|a, b| b.profit.cmp(&a.profit));
    
    msg!("Leaderboard updated. Top PnL: {}", competition.leaderboard[0].profit);
    
    Ok(())
}
