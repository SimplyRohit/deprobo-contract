use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
use anchor_lang::system_program;

declare_id!("9vdmFxJ2L14TWw5BeZWRVpP8w4UcYHpHGPMNRAi5LshH");

pub trait StringExt {
    fn to_hashed_bytes(&self) -> [u8; 32];
}

impl StringExt for String {
    fn to_hashed_bytes(&self) -> [u8; 32] {
        let hash = hash(self.as_bytes());
        hash.to_bytes()
    }
}

#[program]
pub mod prediction_market {
    use super::*;

    pub fn create_market(
        ctx: Context<CreateMarket>,
        question: String,
        close_time: f64,
        category: String,
    ) -> Result<()> {
        let char_count = question.chars().count();
        require!(
            char_count >= 20 && char_count <= 80,
            ErrorCode::InvalidQuestionLength
        );
        let market = &mut ctx.accounts.market;
        let clock = Clock::get()?;
        let time = clock.unix_timestamp;
        let close_time_seconds = (close_time * 60.0 * 60.0) as i64;
        market.created_at = time;
        market.close_time = time + close_time_seconds as i64;
        market.authority = *ctx.accounts.creator.key;
        market.question = question.clone();
        market.yes_pool = ctx.accounts.yes_pool.key();
        market.no_pool = ctx.accounts.no_pool.key();
        market.total_yes = 0;
        market.total_no = 0;
        market.resolved = false;
        market.bet = true;
        market.category = category.clone();
        market.yes_users = 0;
        market.no_users = 0;

        emit!(MarketCreated {
            market: market.key(),
            authority: *ctx.accounts.creator.key,
            question,
            close_time: market.close_time,
            category,
        });
        Ok(())
    }

    pub fn place_bet(ctx: Context<PlaceBet>, amount: u64, outcome: bool) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let user = &ctx.accounts.user;
        let clock = Clock::get()?;
        require!(market.bet, ErrorCode::BettingClosed);

        if clock.unix_timestamp > market.close_time {
            market.bet = false;
            return err!(ErrorCode::BettingClosed);
        }
        if outcome {
            let cpi = CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: user.to_account_info(),
                    to: ctx.accounts.yes_pool.to_account_info(),
                },
            );
            system_program::transfer(cpi, amount)?;
            market.yes_users = market.yes_users.checked_add(1).unwrap();
            market.total_yes = market.total_yes.checked_add(amount).unwrap();
        } else {
            let cpi = CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                system_program::Transfer {
                    from: user.to_account_info(),
                    to: ctx.accounts.no_pool.to_account_info(),
                },
            );
            system_program::transfer(cpi, amount)?;
            market.no_users = market.no_users.checked_add(1).unwrap();
            market.total_no = market.total_no.checked_add(amount).unwrap();
        }
        let bet = &mut ctx.accounts.bet;
        bet.user = *user.key;
        bet.market = market.key();
        bet.amount = amount;
        bet.outcome = outcome;
        bet.claimed = false;

        emit!(BetPlaced {
            market: market.key(),
            user: *user.key,
            amount,
            outcome,
        });
        Ok(())
    }
    pub fn resolve_market(ctx: Context<ResolveMarket>, outcome: bool) -> Result<()> {
        let market = &mut ctx.accounts.market;
        require!(!market.resolved, ErrorCode::AlreadyResolved);
        market.resolved = true;
        market.winning_outcome = outcome;
        emit!(MarketResolved {
            market: market.key(),
            winning_outcome: outcome,
        });
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
}

#[derive(Accounts)]
#[instruction(question: String)]
pub struct CreateMarket<'info> {
    #[account(
        init,
        payer = creator,
        space = 8 + Market::MAX_SIZE,
        seeds = [b"market", creator.key().as_ref(),   &question.to_hashed_bytes()],
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
    pub bet: bool,
    pub category: String,
    pub created_at: i64,
    pub close_time: i64,
    pub question: String,
    pub yes_pool: Pubkey,
    pub no_pool: Pubkey,
    pub total_yes: u64,
    pub total_no: u64,
    pub yes_users: u64,
    pub no_users: u64,
    pub resolved: bool,
    pub winning_outcome: bool,
}
impl Market {
    pub const MAX_SIZE: usize =
        8 + 32 + 1 + 4 + 50 + 8 + 8 + 4 + 800 + 32 + 32 + 8 + 8 + 8 + 8 + 1 + 1;
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

#[event]
pub struct MarketCreated {
    pub market: Pubkey,
    pub authority: Pubkey,
    pub question: String,
    pub close_time: i64,
    pub category: String,
}

#[event]
pub struct BetPlaced {
    pub market: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub outcome: bool,
}

#[event]
pub struct MarketResolved {
    pub market: Pubkey,
    pub winning_outcome: bool,
}

#[event]
pub struct WinningsClaimed {
    pub market: Pubkey,
    pub user: Pubkey,
    pub payout: u64,
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
    #[msg("Betting is closed for this market.")]
    BettingClosed,
    #[msg("Question must be between 20 and 80 words.")]
    InvalidQuestionLength,
}
