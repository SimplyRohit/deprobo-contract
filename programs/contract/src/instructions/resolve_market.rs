use anchor_lang::prelude::*;
use crate::events::*;
use crate::state::*;
use crate::errors::ErrorCode;

pub fn resolve_market(ctx: Context<ResolveMarket>, outcome: bool) -> Result<()> {
    let market = &mut ctx.accounts.market;
    require!(!market.resolved, ErrorCode::AlreadyResolved);

    market.resolved = true;
    market.winning_outcome = outcome;
    market.bet = false;

    let loser_pool = if outcome {
        &ctx.accounts.no_pool
    } else {
        &ctx.accounts.yes_pool
    };

    let authority = &ctx.accounts.authority;

    let loser_balance = **loser_pool.to_account_info().lamports.borrow();
    let fee = loser_balance
        .checked_mul(20)
        .unwrap()
        .checked_div(100)
        .unwrap();

    **loser_pool.to_account_info().try_borrow_mut_lamports()? -= fee;
    **authority.to_account_info().try_borrow_mut_lamports()? += fee;
    market.fee_collected = fee;

    emit!(MarketResolved {
        market: market.key(),
        winning_outcome: outcome,
        fee,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    #[account(mut, has_one = authority)]
    pub market: Account<'info, Market>,

    #[account(mut, address = market.yes_pool)]
    pub yes_pool: Account<'info, PoolAccount>,

    #[account(mut, address = market.no_pool)]
    pub no_pool: Account<'info, PoolAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,
}