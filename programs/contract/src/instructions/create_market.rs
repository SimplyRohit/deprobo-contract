use anchor_lang::prelude::*;

use crate::events::*;
use crate::state::*;

pub fn create_market(
    ctx: Context<CreateMarket>,
    created_at: i64,
    close_time: i64,
    question: String,
    category: String,
) -> Result<()> {
    let market = &mut ctx.accounts.market;
    market.created_at = created_at;
    market.close_time = close_time;
    market.authority = *ctx.accounts.creator.key;
    market.yes_pool = ctx.accounts.yes_pool.key();
    market.no_pool = ctx.accounts.no_pool.key();
    market.total_yes = 0;
    market.total_no = 0;
    market.resolved = false;
    market.bet = true;
    market.winning_outcome = false;
    market.fee_collected = 0;

    emit!(MarketCreated {
        market: market.key(),
        created_at,
        close_time,
        authority: *ctx.accounts.creator.key,
        question,
        category,
    });

    Ok(())
}

#[derive(Accounts)]
#[instruction(created_at: i64)]
pub struct CreateMarket<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + Market::SIZE,
        seeds = [b"market", creator.key().as_ref(), &created_at.to_le_bytes()],
        bump
    )]
    pub market: Account<'info, Market>,

    #[account(
        init,
        payer = creator,
        space = 8,
        seeds = [b"yes_pool", market.key().as_ref()],
        bump
    )]
    pub yes_pool: Account<'info, PoolAccount>,

    #[account(
        init,
        payer = creator,
        space = 8,
        seeds = [b"no_pool", market.key().as_ref()],
        bump
    )]
    pub no_pool: Account<'info, PoolAccount>,

    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
}