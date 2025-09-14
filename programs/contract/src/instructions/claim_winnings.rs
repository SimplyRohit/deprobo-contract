use anchor_lang::prelude::*;
use crate::events::*;
use crate::state::*;
use crate::errors::ErrorCode;

pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
    let market = &ctx.accounts.market;
    let bet = &mut ctx.accounts.bet;
    let user = &ctx.accounts.user;

    require!(market.resolved, ErrorCode::NotResolved);
    require!(!bet.claimed, ErrorCode::AlreadyClaimed);
    require!(bet.outcome == market.winning_outcome, ErrorCode::WrongBet);
    require!(bet.market == market.key(), ErrorCode::MarketMismatch);

    let (winner_pool, loser_pool, total_winner_bets) = if market.winning_outcome {
        (
            &ctx.accounts.yes_pool,
            &ctx.accounts.no_pool,
            market.total_yes,
        )
    } else {
        (
            &ctx.accounts.no_pool,
            &ctx.accounts.yes_pool,
            market.total_no,
        )
    };

    let loser_balance = **loser_pool.to_account_info().lamports.borrow();
    let share = loser_balance
        .checked_mul(bet.amount)
        .unwrap()
        .checked_div(total_winner_bets)
        .unwrap();
    let payout = bet.amount.checked_add(share).unwrap();

    **winner_pool.to_account_info().try_borrow_mut_lamports()? -= bet.amount;
    **loser_pool.to_account_info().try_borrow_mut_lamports()? -= share;
    **user.to_account_info().try_borrow_mut_lamports()? += payout;

    bet.claimed = true;

    emit!(WinningsClaimed {
        market: market.key(),
        user: *user.key,
        payout,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct ClaimWinnings<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,

    #[account(mut, address = market.yes_pool)]
    pub yes_pool: Account<'info, PoolAccount>,

    #[account(mut, address = market.no_pool)]
    pub no_pool: Account<'info, PoolAccount>,

    #[account(mut, has_one = user)]
    pub bet: Account<'info, BetAccount>,

    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}