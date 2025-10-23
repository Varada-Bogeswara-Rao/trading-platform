use anchor_lang::prelude::*;
use crate::competition::*;
use crate::events::WinnerNftMinted;
use anchor_spl::token::{Mint, Token, TokenAccount};
use mpl_token_metadata::instructions::{CreateMetadataAccountV3, CreateMetadataAccountV3InstructionArgs};
use mpl_token_metadata::types::{Creator, DataV2};

#[derive(Accounts)]
#[instruction(metadata_uri: String)]
pub struct MintWinnerNft<'info> {
    #[account(
        mut,
        has_one = authority,
        has_one = winner,
        constraint = competition.phase == CompetitionPhase::Settled @ CompetitionError::NotActive
    )]
    pub competition: Account<'info, Competition>,

    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        has_one = competition,
        has_one = user,
        constraint = winner_position.user == competition.winner @ CompetitionError::Unauthorized,
        seeds = [b"position", competition.key().as_ref(), winner_position.user.as_ref()],
        bump = winner_position.bump
    )]
    pub winner_position: Account<'info, Position>,

    /// CHECK: Verified by constraint
    #[account(address = winner_position.user @ CompetitionError::Unauthorized)]
    pub winner_wallet: UncheckedAccount<'info>,

    #[account(
        init,
        payer = authority,
        mint::decimals = 0,
        mint::authority = competition,
        seeds = [b"nft_mint", competition.key().as_ref(), winner_position.user.as_ref()],
        bump
    )]
    pub nft_mint: Account<'info, Mint>,

    #[account(
        init,
        payer = authority,
        token::mint = nft_mint,
        token::authority = winner_wallet
    )]
    pub nft_account: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [
            b"metadata",
            mpl_token_metadata::ID.to_bytes().as_ref(),
            nft_mint.key().as_ref()
        ],
        bump,
        seeds::program = token_metadata_program
    )]
    /// CHECK: Metaplex metadata PDA
    pub metadata_account: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    #[account(address = mpl_token_metadata::ID)]
    pub token_metadata_program: UncheckedAccount<'info>,
    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

pub fn handler(ctx: Context<MintWinnerNft>, metadata_uri: String) -> Result<()> {
    let comp = &ctx.accounts.competition;
    let pos = &ctx.accounts.winner_position;

    let signer_seeds: &[&[&[u8]]] = &[&[
        b"competition",
        comp.authority.as_ref(),
        &[comp.bump]
    ]];

    // Mint the NFT
    anchor_spl::token::mint_to(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            anchor_spl::token::MintTo {
                mint: ctx.accounts.nft_mint.to_account_info(),
                to: ctx.accounts.nft_account.to_account_info(),
                authority: ctx.accounts.competition.to_account_info(),
            },
            signer_seeds,
        ),
        1,
    )?;

    // Create Metaplex metadata
    let creator = Creator {
        address: comp.key(),
        verified: true,
        share: 100,
    };
    let name = format!("Cypherpunk Winner: ${}", pos.profit())
        .chars()
        .take(32)
        .collect::<String>();

    let data = DataV2 {
        name,
        symbol: "CYPHR".to_string(),
        uri: metadata_uri,
        seller_fee_basis_points: 0,
        creators: Some(vec![creator]),
        collection: None,
        uses: None,
    };

    let ix = CreateMetadataAccountV3 {
        metadata: ctx.accounts.metadata_account.key(),
        mint: ctx.accounts.nft_mint.key(),
        mint_authority: comp.key(),
        payer: ctx.accounts.authority.key(),
        update_authority: (comp.key(), true),
        system_program: ctx.accounts.system_program.key(),
        rent: Some(ctx.accounts.rent.key()),
    }
    .instruction(CreateMetadataAccountV3InstructionArgs {
        data,
        is_mutable: true,
        collection_details: None,
    });

    anchor_lang::solana_program::program::invoke_signed(
        &ix,
        &[
            ctx.accounts.metadata_account.to_account_info(),
            ctx.accounts.nft_mint.to_account_info(),
            ctx.accounts.competition.to_account_info(),
            ctx.accounts.authority.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
            ctx.accounts.token_metadata_program.to_account_info(),
            ctx.accounts.rent.to_account_info(),
        ],
        signer_seeds,
    )?;

    // Emit event
    emit!(WinnerNftMinted {
        winner: pos.user,
        competition: comp.key(),
        nft_mint: ctx.accounts.nft_mint.key(),
        profit: pos.profit(),
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}