use anchor_lang::prelude::*;
use crate::competition::{Competition, CompetitionError};
use anchor_spl::token::{Mint, Token};
use mpl_token_metadata::{
    instruction::{create_metadata_accounts_v3},
    state::Creator,
};
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::system_program::ID as SYSTEM_PROGRAM_ID;
use anchor_lang::solana_program::sysvar::rent::Rent;

#[derive(Accounts)]
#[instruction(metadata_uri: String)]
pub struct MintWinnerNft<'info> {
    // The Competition account must be a PDA to sign for the NFT metadata creation
    #[account(mut, has_one = authority)]
    pub competition: Account<'info, Competition>,
    
    // Admin Authority who signs the transaction to initialize the minting process
    #[account(mut)]
    pub authority: Signer<'info>, 
    
    // Winner account (must be the actual winner based on the committed state)
    /// CHECK: We check if this account is the rightful winner against the leaderboard.
    pub winner: AccountInfo<'info>, 
    
    // Accounts for NFT Minting (must be pre-created by the client)
    #[account(mut)]
    pub nft_mint: Account<'info, Mint>,
    #[account(mut)]
    pub nft_account: Account<'info, anchor_spl::token::TokenAccount>,
    
    // Metaplex Metadata PDA (must be the correct PDA derived from the NFT mint)
    #[account(mut)]
    /// CHECK: The metadata PDA address is verified implicitly by Metaplex CPI
    pub metadata_account: UncheckedAccount<'info>,
    
    // Programs
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    #[account(address = mpl_token_metadata::id())]
    /// CHECK: Address checked against Metaplex ID
    pub token_metadata_program: UncheckedAccount<'info>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<MintWinnerNft>,
    metadata_uri: String,
) -> Result<()> {
    let competition = &ctx.accounts.competition;
    
    // 1. Pre-Check: Ensure the competition session is finalized.
    require!(!competition.is_active, CompetitionError::NotActive);

    // 2. Winner Verification: Read the finalized leaderboard state.
    let winner_key = ctx.accounts.winner.key();
    let top_position = competition.leaderboard.first().ok_or(CompetitionError::NoWinner)?;
    
    // This check uses the data finalized on L1 after the MagicBlock State Commitment.
    require!(top_position.user == winner_key, CompetitionError::Unauthorized);

    // --- CPI Signer Seeds ---
    let (competition_key, competition_bump) = Pubkey::find_program_address(
        &[b"competition", ctx.accounts.authority.key().as_ref()],
        ctx.program_id,
    );
    let signer_seeds: &[&[&[u8]]] = &[
        &[
            b"competition",
            ctx.accounts.authority.key().as_ref(),
            &[competition_bump],
        ],
    ];

    // 3. Mint NFT (CPI to SPL Token Program)
    // The Competition PDA acts as the Mint Authority, so we use invoke_signed.
    anchor_spl::token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::MintTo {
                mint: ctx.accounts.nft_mint.to_account_info(),
                to: ctx.accounts.nft_account.to_account_info(),
                // The Competition PDA is the authority
                authority: ctx.accounts.competition.to_account_info(), 
            },
            signer_seeds,
        ),
        1, // Mint exactly 1 NFT
    )?;
    
    // 4. Create Metadata (CPI to Metaplex Token Metadata Program)
    let creator = Creator {
        address: competition_key, // The Competition PDA is the creator
        verified: true,
        share: 100,
    };
    
    let metadata_instruction = create_metadata_accounts_v3(
        ctx.accounts.token_metadata_program.key(),
        ctx.accounts.metadata_account.key(),
        ctx.accounts.nft_mint.key(),
        competition_key, // Mint Authority / Update Authority (Competition PDA)
        ctx.accounts.authority.key(), // Payer (Admin)
        competition_key, // Update Authority
        format!("Competition Winner - {}s", competition.duration),
        "WINNER",
        Some(metadata_uri), // Your URI from instruction data
        Some(vec![creator]),
        0,
        true, // Is mutable
        true, // Is master edition
        None,
        None,
        None,
    );

    // Invoke the Metaplex instruction, signed by the Competition PDA
    invoke_signed(
        &metadata_instruction,
        &[
            ctx.accounts.metadata_account.to_account_info(),
            ctx.accounts.nft_mint.to_account_info(),
            ctx.accounts.competition.to_account_info(), // Mint Authority (PDA)
            ctx.accounts.authority.to_account_info(), // Payer (Signer)
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_metadata_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
        ],
        signer_seeds,
    )?;

    Ok(())
}
