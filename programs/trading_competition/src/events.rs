use anchor_lang::prelude::*;

#[event]
#[derive(Copy, Clone)]
pub struct TradeExecuted {
    pub user: Pubkey,
    pub competition: Pubkey,
    pub timestamp: i64,
    pub is_buy: bool,
    pub amount_u64: u64,
    pub new_balance: u128,
    pub new_current_value: u128,
    pub new_profit: i128,
    pub price_used: i64,
}

#[event]
#[derive(Copy, Clone)]
pub struct WinnerNftMinted {
    pub winner: Pubkey,
    pub competition: Pubkey,
    pub nft_mint: Pubkey,
    pub profit: i128,
    pub timestamp: i64,
}

#[event]
#[derive(Copy, Clone)]
pub struct UserUndelegated {
    pub user: Pubkey,
    pub competition: Pubkey,
    pub final_pnl: i128,
    pub timestamp: i64,
}