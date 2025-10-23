use anchor_lang::prelude::*;

/// Phase of a competition – replaces the old `is_active` bool.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum CompetitionPhase {
    Upcoming,
    Active,
    Finalizing,   // after final_commit, challenge window open
    Settled,      // after settle_competition, safe to mint NFT
}

// Manual Space impl for enum (Anchor requires it for InitSpace on containing accounts)
impl anchor_lang::Space for CompetitionPhase {
    const INIT_SPACE: usize = 1;  // u8 discriminant size
}

#[account]
#[derive(InitSpace)]
pub struct Competition {
    pub authority: Pubkey,          // admin
    pub usdc_mint: Pubkey,
    pub er_instance: Pubkey,        // MagicBlock ER instance
    pub mock_price_pda: Pubkey,     // PDA for MockPriceAccount
    pub start_time: i64,
    pub end_time: i64,              // absolute timestamp
    pub phase: CompetitionPhase,
    pub state_root: [u8; 32],       // Merkle root of final ER state
    pub winner: Pubkey,             // set in final_commit
    pub winner_profit: i128,
    pub challenge_deadline: i64,    // unix ts when challenge window closes
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Position {
    pub competition: Pubkey,
    pub user: Pubkey,
    pub usdc_ata: Pubkey,           // delegated ATA (USDC)
    pub usdc_balance: u128,         // synthetic USDC balance (6-dec)
    pub initial_value: u128,
    pub current_value: u128,
    pub bump: u8,
}

impl Position {
    /// Profit is **derived** – never stored.
    pub fn profit(&self) -> i128 {
        self.current_value as i128 - self.initial_value as i128
    }
}

#[account]
#[derive(InitSpace)]
pub struct MockPriceAccount {
    pub price: u128,                // Normalized price (e.g., 150_000_000 for $150 with 6 decimals)
    pub expo: i32,                  // Exponent for precision (e.g., -8 for 10^-8 scaling)
    pub timestamp: i64,             // Last update Unix timestamp
    pub bump: u8,                   // PDA bump
}

impl MockPriceAccount {
    pub fn update_price(&mut self, new_price: u128, new_expo: i32, clock: &Clock) -> Result<()> {
        self.price = new_price;
        self.expo = new_expo;
        self.timestamp = clock.unix_timestamp;
        Ok(())
    }
}

#[error_code]
pub enum CompetitionError {
    #[msg("Competition not active")]
    NotActive,
    #[msg("Invalid price feed")]
    InvalidPriceFeed,
    #[msg("Stale price feed")]
    StalePrice,
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Competition not ended")]
    NotEnded,
    #[msg("No winner determined")]
    NoWinner,
    #[msg("User already registered")]
    UserAlreadyRegistered,
    #[msg("Account must be delegated to ER")]
    AccountNotDelegated,
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Arithmetic overflow")]
    CalculationError,
}