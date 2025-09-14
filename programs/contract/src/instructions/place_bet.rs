use anchor_lang::prelude::*;
use anchor_lang::system_program;
use crate::events::*;
use crate::state::*;
use crate::errors::ErrorCode;

pub fn place_bet(ctx: Context<PlaceBet>, amount: u64, outcome: bool) -> Result<()> {
    let market = &mut ctx.accounts.market;
    let user = &ctx.accounts.user;
    let clock = Clock::get()?;

    require!(market.bet, ErrorCode::BettingClosed);

    if clock.unix_timestamp > market.close_time {
        market.bet = false;
        return err!(ErrorCode::BettingClosed);
    }

    require!(
        amount >= 1 && amount <= 10 * anchor_lang::solana_program::native_token::LAMPORTS_PER_SOL,
        ErrorCode::BetAmountInvalid
    );

    let transfer_target = if outcome {
        ctx.accounts.yes_pool.to_account_info()
    } else {
        ctx.accounts.no_pool.to_account_info()
    };

    let cpi = CpiContext::new(
        ctx.accounts.system_program.to_account_info(),
        system_program::Transfer {
            from: user.to_account_info(),
            to: transfer_target,
        },
    );
    system_program::transfer(cpi, amount)?;

    if outcome {
        market.total_yes = market.total_yes.checked_add(amount).unwrap();
    } else {
        market.total_no = market.total_no.checked_add(amount).unwrap();
    }

    let bet = &mut ctx.accounts.bet;
    bet.user = *user.key;
    bet.market = market.key();
    bet.amount = amount;
    bet.outcome = outcome;
    bet.claimed = false;

    emit!(BetPlaced {
        market: bet.market,
        user: bet.user,
        amount,
        outcome,
    });

    Ok(())
}

#[derive(Accounts)]
pub struct PlaceBet<'info> {
    #[account(mut)]
    pub market: Account<'info, Market>,

    #[account(
        init,
        payer = user,
        space = 8 + BetAccount::SIZE,
        seeds = [b"bet", user.key().as_ref(), market.key().as_ref()],
        bump
    )]
    pub bet: Account<'info, BetAccount>,

    #[account(mut, address = market.yes_pool)]
    pub yes_pool: Account<'info, PoolAccount>,

    #[account(mut, address = market.no_pool)]
    pub no_pool: Account<'info, PoolAccount>,

    #[account(mut)]
    pub user: Signer<'info>,
    pub system_program: Program<'info, System>,
}