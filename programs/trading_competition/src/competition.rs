use anchor_lang::prelude::*;

#[account]
pub struct Competition {
    pub authority: Pubkey,        // Admin authority
    pub usdc_mint: Pubkey,         // Fake USDC mint address
    pub start_time: i64,           // Unix timestamp for competition start
    pub duration: i64,             // Duration in seconds (e.g., 180 for 3min)
    pub users: Vec<Pubkey>,        // Participating user wallets
    pub leaderboard: Vec<Position>, // Sorted by profit
    pub is_active: bool,           // Competition status
}

#[account]
pub struct Position {
    pub user: Pubkey,
    pub usdc_balance: u64,         // Fake USDC balance
    pub initial_value: u64,        // Initial portfolio value
    pub current_value: u64,        // Current portfolio value (based on trades)
    pub profit: i64,               // Profit/loss (signed)
}

#[error_code]
pub enum CompetitionError {
    #[msg("Competition is not active")]
    NotActive,
    #[msg("Invalid price feed")]
    InvalidPriceFeed,
    #[msg("Unauthorized user")]
    Unauthorized,
    #[msg("Competition not ended")]
    NotEnded,
    #[msg("No winner determined")]
    NoWinner,
}
