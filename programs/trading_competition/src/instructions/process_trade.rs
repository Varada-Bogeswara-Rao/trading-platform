use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::clock::Clock;
use anchor_spl::token::TokenAccount;
use anchor_lang::solana_program::program_option::COption;

use crate::competition::{Competition, CompetitionError, CompetitionPhase, MockPriceAccount, Position};
use crate::events::TradeExecuted;

#[derive(Accounts)]
#[instruction(amount: u64, is_buy: bool)]
pub struct ProcessTrade<'info> {
    #[account(has_one = er_instance)]
    pub competition: Account<'info, Competition>,

    #[account(
        mut,
        has_one = user,
        has_one = competition,
        seeds = [b"position", competition.key().as_ref(), user.key().as_ref()],
        bump = position.bump,
    )]
    pub position: Account<'info, Position>,

    #[account(
        constraint = usdc_ata.delegate == COption::Some(competition.er_instance) @ CompetitionError::AccountNotDelegated
    )]
    pub usdc_ata: Account<'info, TokenAccount>,

    pub user: Signer<'info>,

    /// CHECK: Verified through `competition.er_instance`
    #[account(address = competition.er_instance @ CompetitionError::Unauthorized)]
    pub er_instance: UncheckedAccount<'info>,

    #[account(
        seeds = [b"mock_price", competition.key().as_ref()],
        bump = mock_price.bump,
    )]
    pub mock_price: Account<'info, MockPriceAccount>,

    pub clock: Sysvar<'info, Clock>,
}

pub fn handler(
    ctx: Context<ProcessTrade>,
    amount: u64,
    is_buy: bool,
) -> Result<()> {
    let comp = &ctx.accounts.competition;
    let pos = &mut ctx.accounts.position;
    let mock_price = &ctx.accounts.mock_price;
    let clock = Clock::get()?;
    let now = clock.unix_timestamp;

    // ---- Phase and time validation ----
    require!(comp.phase == CompetitionPhase::Active, CompetitionError::NotActive);
    require!(now < comp.end_time, CompetitionError::NotEnded);
    require!(amount >= 100_000, CompetitionError::InsufficientFunds); // ≥ 0.1 USDC

    // ---- Load price from Mock Oracle ----
    require!(
        now - mock_price.timestamp <= 15,  // Mock "freshness" check (<15s)
        CompetitionError::StalePrice
    );

    let current_price = mock_price.price;
    let expo = mock_price.expo.abs() as u32;

    // ---- Normalize price to 6 decimals (USDC) ----
    let price_norm = current_price
        .checked_mul(1_000_000u128) // scale to 6 decimals
        .ok_or(CompetitionError::CalculationError)?
        .checked_div(
            10u128
                .checked_pow(expo)
                .ok_or(CompetitionError::CalculationError)?,
        )
        .ok_or(CompetitionError::CalculationError)?;

    // ---- Compute trade value ----
    let trade_value = (amount as u128)
        .checked_mul(price_norm)
        .ok_or(CompetitionError::CalculationError)?
        .checked_div(1_000_000u128) // adjust for 6-dec scaling
        .ok_or(CompetitionError::CalculationError)?;

    // ---- Execute trade ----
    if is_buy {
        // Buy asset → spend USDC
        pos.usdc_balance = pos.usdc_balance
            .checked_sub(trade_value)
            .ok_or(CompetitionError::InsufficientFunds)?;
        pos.current_value = pos.current_value
            .checked_add(trade_value)
            .ok_or(CompetitionError::CalculationError)?;
    } else {
        // Sell asset → receive USDC
        pos.usdc_balance = pos.usdc_balance
            .checked_add(trade_value)
            .ok_or(CompetitionError::CalculationError)?;
        pos.current_value = pos.current_value
            .checked_sub(trade_value)
            .ok_or(CompetitionError::InsufficientFunds)?;
    }

    // ---- Emit event ----
    emit!(TradeExecuted {
        user: pos.user,
        competition: comp.key(),
        timestamp: now,
        is_buy,
        amount_u64: amount,
        new_balance: pos.usdc_balance,
        new_current_value: pos.current_value,
        new_profit: pos.profit(),  // Fixed: Now works on &Position
        price_used: current_price as i64,  // Cast for event
    });

    Ok(())
}





// use anchor_lang::prelude::*;
// use anchor_lang::solana_program::sysvar::clock::Clock;
// use anchor_spl::token::TokenAccount;
// use anchor_lang::solana_program::program_option::COption; // Updated import

// use crate::competition::{Competition, CompetitionError, CompetitionPhase, Position};
// use crate::events::TradeExecuted;

// use pyth_sdk_solana::{load_price_feed_from_account_info, PriceFeed};

// #[derive(Accounts)]
// #[instruction(amount: u64, is_buy: bool, price_feed_id: Pubkey)]
// pub struct ProcessTrade<'info> {
//     #[account(has_one = er_instance)]
//     pub competition: Account<'info, Competition>,

//     #[account(
//         mut,
//         has_one = user,
//         has_one = competition,
//         seeds = [b"position", competition.key().as_ref(), user.key().as_ref()],
//         bump = position.bump,
//     )]
//     pub position: Account<'info, Position>,

//     #[account(
//         constraint = usdc_ata.delegate == COption::Some(competition.er_instance) @ CompetitionError::AccountNotDelegated
//     )]
//     pub usdc_ata: Account<'info, TokenAccount>,

//     pub user: Signer<'info>,

//     /// CHECK: Verified through `competition.er_instance`
//     #[account(address = competition.er_instance @ CompetitionError::Unauthorized)]
//     pub er_instance: UncheckedAccount<'info>,

//     /// CHECK: Pyth price feed account (validated via load_price_feed_from_account_info)
//     pub price_feed: UncheckedAccount<'info>,

//     pub clock: Sysvar<'info, Clock>,
// }

// pub fn handler(
//     ctx: Context<ProcessTrade>,
//     amount: u64,
//     is_buy: bool,
//     price_feed_id: Pubkey,
// ) -> Result<()> {
//     let comp = &ctx.accounts.competition;
//     let pos = &mut ctx.accounts.position;
//     let clock = Clock::get()?;
//     let now = clock.unix_timestamp;

//     // ---- Phase and time validation ----
//     require!(comp.phase == CompetitionPhase::Active, CompetitionError::NotActive);
//     require!(now < comp.end_time, CompetitionError::NotEnded);
//     require!(amount >= 100_000, CompetitionError::InsufficientFunds); // ≥ 0.1 USDC

//     // ---- Load price from Pyth (legacy SDK) ----
//     let price_feed = load_price_feed_from_account_info(&ctx.accounts.price_feed.to_account_info())
//         .map_err(|_| error!(CompetitionError::InvalidPriceFeed))?;
//     let current_price = price_feed
//         .get_price_no_older_than(&clock, 15)
//         .map_err(|_| error!(CompetitionError::StalePrice))?;

//     // ---- Normalize price to 6 decimals (USDC) ----
//     // Example: price = 50.1234, expo = -4 → normalized = 50_123_400
//     let price_u128 = current_price.price.unsigned_abs() as u128;
//     let expo = current_price.expo.abs() as u32;

//     let price_norm = price_u128
//         .checked_mul(1_000_000u128) // scale to 6 decimals
//         .ok_or(CompetitionError::CalculationError)?
//         .checked_div(
//             10u128
//                 .checked_pow(expo)
//                 .ok_or(CompetitionError::CalculationError)?,
//         )
//         .ok_or(CompetitionError::CalculationError)?;

//     // ---- Compute trade value ----
//     let trade_value = (amount as u128)
//         .checked_mul(price_norm)
//         .ok_or(CompetitionError::CalculationError)?
//         .checked_div(1_000_000u128) // adjust for 6-dec scaling
//         .ok_or(CompetitionError::CalculationError)?;

//     // ---- Execute trade ----
//     if is_buy {
//         // Buy asset → spend USDC
//         pos.usdc_balance = pos.usdc_balance
//             .checked_sub(trade_value)
//             .ok_or(CompetitionError::InsufficientFunds)?;
//         pos.current_value = pos.current_value
//             .checked_add(trade_value)
//             .ok_or(CompetitionError::CalculationError)?;
//     } else {
//         // Sell asset → receive USDC
//         pos.usdc_balance = pos.usdc_balance
//             .checked_add(trade_value)
//             .ok_or(CompetitionError::CalculationError)?;
//         pos.current_value = pos.current_value
//             .checked_sub(trade_value)
//             .ok_or(CompetitionError::InsufficientFunds)?;
//     }

//     // ---- Emit event ----
//     emit!(TradeExecuted {
//         user: pos.user,
//         competition: comp.key(),
//         timestamp: now,
//         is_buy,
//         amount_u64: amount,
//         new_balance: pos.usdc_balance,
//         new_current_value: pos.current_value,
//         new_profit: pos.profit(),
//         price_used: current_price.price,
//     });

//     Ok(())
// }