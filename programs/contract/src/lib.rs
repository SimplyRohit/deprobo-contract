use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
use anchor_lang::system_program;

declare_id!("9vdmFxJ2L14TWw5BeZWRVpP8w4UcYHpHGPMNRAi5LshH");

pub trait StringExt {
    fn to_hashed_bytes(&self) -> [u8; 32];
}

impl StringExt for String {
    fn to_hashed_bytes(&self) -> [u8; 32] {
        hash(self.as_bytes()).to_bytes()
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
        let market = &mut ctx.accounts.market;
        let now = Clock::get()?.unix_timestamp;

        let char_count = question.chars().count();
        require!(
            char_count >= 20 && char_count <= 75,
            ErrorCode::InvalidQuestionLength
        );

        market.authority = *ctx.accounts.creator.key;
        market.created_at = now;
        market.close_time = now + (close_time * 3600.0) as i64;
        market.question = question.clone();
        market.category = category.clone();
        market.yes_pool = ctx.accounts.yes_pool.key();
        market.no_pool = ctx.accounts.no_pool.key();
        market.total_yes = 0;
        market.total_no = 0;
        market.resolved = false;
        market.bet_open = true;

        emit!(MarketCreated {
            market: market.key(),
            authority: market.authority,
            question,
            created_at: market.created_at,
            close_time: market.close_time,
            yes_pool: market.yes_pool,
            no_pool: market.no_pool,
            total_no: market.total_no,
            total_yes: market.total_yes,
            category,
        });

        Ok(())
    }

    pub fn place_bet(ctx: Context<PlaceBet>, amount: u64, outcome: bool) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let user = &ctx.accounts.user;

        require!(market.bet_open, ErrorCode::BettingClosed);
        require!(
            Clock::get()?.unix_timestamp <= market.close_time,
            ErrorCode::BettingClosed
        );

        let cpi = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: user.to_account_info(),
                to: if outcome {
                    ctx.accounts.yes_pool.to_account_info()
                } else {
                    ctx.accounts.no_pool.to_account_info()
                },
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
            total_yes: market.total_yes,
            total_no: market.total_no,
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

        let (winner_pool, loser_pool, total_winner) = if market.winning_outcome {
            (&ctx.accounts.yes_pool, &ctx.accounts.no_pool, market.total_yes)
        } else {
            (&ctx.accounts.no_pool, &ctx.accounts.yes_pool, market.total_no)
        };

        require!(total_winner > 0, ErrorCode::NoWinningPool);

        let loser_balance = **loser_pool.to_account_info().lamports.borrow();
        let share = loser_balance.checked_mul(bet.amount).unwrap() / total_winner;
        let payout = bet.amount + share;

        **winner_pool.to_account_info().try_borrow_mut_lamports()? -= bet.amount;
        **loser_pool.to_account_info().try_borrow_mut_lamports()? -= share;
        **user.to_account_info().try_borrow_mut_lamports()? += payout;

        bet.claimed = true;

        emit!(WinningsClaimed {
            market: market.key(),
            user: bet.user,
            payout,
            bet_claimed: bet.claimed,
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
        space = 8 + Market::SIZE,
        seeds = [b"market", creator.key().as_ref(), &question.to_hashed_bytes()],
        bump
    )]
    pub market: Account<'info, Market>,
    #[account(init, payer = creator, seeds = [b"yes_pool", market.key().as_ref()], bump, space = 8)]
    pub yes_pool: Account<'info, PoolAccount>,
    #[account(init, payer = creator, seeds = [b"no_pool", market.key().as_ref()], bump, space = 8)]
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
        seeds = [b"bet", user.key().as_ref(), market.key().as_ref()],
        bump,
        space = 8 + BetAccount::SIZE
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
    pub created_at: i64,
    pub close_time: i64,
    pub question: String,
    pub category: String,
    pub yes_pool: Pubkey,
    pub no_pool: Pubkey,
    pub total_yes: u64,
    pub total_no: u64,
    pub resolved: bool,
    pub bet_open: bool,
    pub winning_outcome: bool,
}
impl Market {
    pub const SIZE: usize = 32 + 8 + 8 + 4 + 75 + 4 + 30 + 32 + 32 + 8 + 8 + 1 + 1 + 1;
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
    pub created_at: i64,
    pub close_time: i64,
    pub yes_pool: Pubkey,
    pub no_pool: Pubkey,
    pub total_no: u64,
    pub total_yes: u64,
    pub category: String,
}

#[event]
pub struct BetPlaced {
    pub market: Pubkey,
    pub user: Pubkey,
    pub amount: u64,
    pub outcome: bool,
    pub total_yes: u64,
    pub total_no: u64,
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
    pub bet_claimed: bool,
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
    #[msg("Question must be between 20 and 75 characters.")]
    InvalidQuestionLength,
    #[msg("No winning pool balance to distribute.")]
    NoWinningPool,
}
