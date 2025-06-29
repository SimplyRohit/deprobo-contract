use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("JAVuBXeBZqXNtS73azhBDAoYaaAFfo4gWXoZe2e7Jf8H");

#[program]
pub mod prediction_market {
    use super::*;

    pub fn create_market(ctx: Context<CreateMarket>, question: String, close_time: i64) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let clock = Clock::get()?;
        require!(close_time > clock.unix_timestamp, ErrorCode::InvalidCloseTime);
        require!(question.len() >= 20 && question.len() <= 400, ErrorCode::InvalidQuestionLength);
        market.authority = *ctx.accounts.creator.key;
        market.question = question;
        market.yes_pool = ctx.accounts.yes_pool.key();
        market.no_pool = ctx.accounts.no_pool.key();
        market.total_yes = 0;
        market.total_no = 0;
        market.resolved = false;
        market.winning_outcome = false;
        market.created_at = clock.unix_timestamp;
        market.close_time = close_time;
        msg!("Instruction: CreateMarket");
        msg!("Market: {:?}", market.key());
        msg!("Question: {}", market.question);
        msg!("Close Time: {}", market.close_time);
        Ok(())
    }

    pub fn place_bet(ctx: Context<PlaceBet>, amount: u64, outcome: bool) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let clock = Clock::get()?;
        require!(clock.unix_timestamp <= market.close_time, ErrorCode::BettingClosed);
        require!(!market.resolved, ErrorCode::AlreadyResolved);
        let user = &ctx.accounts.user;
        if outcome {
            let cpi = CpiContext::new(ctx.accounts.system_program.to_account_info(), system_program::Transfer {
                from: user.to_account_info(),
                to: ctx.accounts.yes_pool.to_account_info(),
            });
            system_program::transfer(cpi, amount)?;
            market.total_yes = market.total_yes.checked_add(amount).ok_or(ErrorCode::Overflow)?;
        } else {
            let cpi = CpiContext::new(ctx.accounts.system_program.to_account_info(), system_program::Transfer {
                from: user.to_account_info(),
                to: ctx.accounts.no_pool.to_account_info(),
            });
            system_program::transfer(cpi, amount)?;
            market.total_no = market.total_no.checked_add(amount).ok_or(ErrorCode::Overflow)?;
        }
        let bet = &mut ctx.accounts.bet;
        bet.user = *user.key;
        bet.market = market.key();
        bet.amount = amount;
        bet.outcome = outcome;
        bet.claimed = false;
        msg!("Instruction: PlaceBet");
        msg!("Market: {:?}", market.key());
        msg!("User: {:?}", bet.user);
        msg!("Outcome: {:?}", outcome);
        msg!("Amount: {:?}", amount);
        Ok(())
    }

    pub fn resolve_market(ctx: Context<ResolveMarket>, outcome: bool) -> Result<()> {
        let market = &mut ctx.accounts.market;
        require!(!market.resolved, ErrorCode::AlreadyResolved);
        require!(market.authority == *ctx.accounts.authority.key, ErrorCode::Unauthorized);
        market.resolved = true;
        market.winning_outcome = outcome;
        msg!("Instruction: ResolveMarket");
        msg!("Market: {:?}", market.key());
        msg!("Outcome: {:?}", outcome);
        Ok(())
    }

    pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
        let market = &ctx.accounts.market;
        let bet = &mut ctx.accounts.bet;
        let user = &ctx.accounts.user;
        require!(market.resolved, ErrorCode::NotResolved);
        require!(!bet.claimed, ErrorCode::AlreadyClaimed);
        require!(bet.outcome == market.winning_outcome, ErrorCode::WrongBet);
        let (winner_pool, loser_pool, total_winner_bets) = if market.winning_outcome {
            (&ctx.accounts.yes_pool, &ctx.accounts.no_pool, market.total_yes)
        } else {
            (&ctx.accounts.no_pool, &ctx.accounts.yes_pool, market.total_no)
        };
        let loser_balance = **loser_pool.to_account_info().lamports.borrow();
        let share = loser_balance
            .checked_mul(bet.amount)
            .ok_or(ErrorCode::Overflow)?
            .checked_div(total_winner_bets)
            .ok_or(ErrorCode::Overflow)?;
        let payout = bet.amount.checked_add(share).ok_or(ErrorCode::Overflow)?;
        **winner_pool.to_account_info().try_borrow_mut_lamports()? -= bet.amount;
        **loser_pool.to_account_info().try_borrow_mut_lamports()? -= share;
        **user.to_account_info().try_borrow_mut_lamports()? += payout;
        bet.claimed = true;
        msg!("Instruction: ClaimWinnings");
        msg!("Market: {:?}", market.key());
        msg!("User: {:?}", bet.user);
        msg!("Payout: {:?}", payout);
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(question: String)]
pub struct CreateMarket<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + Market::MAX_SIZE,
        seeds = [b"market", creator.key().as_ref(), question.as_bytes()],
        bump
    )]
    pub market: Account<'info, Market>,
    #[account(init, payer = creator, space = 8, seeds = [b"yes_pool", market.key().as_ref()], bump)]
    pub yes_pool: Account<'info, PoolAccount>,
    #[account(init, payer = creator, space = 8, seeds = [b"no_pool", market.key().as_ref()], bump)]
    pub no_pool: Account<'info, PoolAccount>,
    #[account(mut)]
    pub creator: Signer<'info>,
    pub system_program: Program<'info, System>,
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

#[derive(Accounts)]
pub struct ResolveMarket<'info> {
    #[account(mut, has_one = authority)]
    pub market: Account<'info, Market>,
    pub authority: Signer<'info>,
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

#[account]
pub struct Market {
    pub authority: Pubkey,
    pub question: String,
    pub yes_pool: Pubkey,
    pub no_pool: Pubkey,
    pub total_yes: u64,
    pub total_no: u64,
    pub resolved: bool,
    pub winning_outcome: bool,
    pub created_at: i64,
    pub close_time: i64,
}

impl Market {
    pub const MAX_SIZE: usize = 32 + 4 + 400 + 32 + 32 + 8 + 8 + 1 + 1 + 8 + 8;
}

#[account]
pub struct PoolAccount {}

#[account]
pub struct BetAccount {
    pub user: Pubkey,
    pub market: Pubkey,
    pub amount: u64,
    pub outcome: bool,
    pub claimed: bool,
}

impl BetAccount {
    pub const SIZE: usize = 32 + 32 + 8 + 1 + 1;
}

#[error_code]
pub enum ErrorCode {
    #[msg("Market already resolved")]
    AlreadyResolved,
    #[msg("Market not yet resolved")]
    NotResolved,
    #[msg("Winnings already claimed")]
    AlreadyClaimed,
    #[msg("Bet outcome is not the winner")]
    WrongBet,
    #[msg("Betting for this market is closed")]
    BettingClosed,
    #[msg("Invalid close time")]
    InvalidCloseTime,
    #[msg("Overflow error")]
    Overflow,
    #[msg("Unauthorized resolver")]
    Unauthorized,
    #[msg("Question length must be between 20 and 400 characters")]
    InvalidQuestionLength,

}
