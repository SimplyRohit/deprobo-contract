use anchor_lang::prelude::*;

declare_id!("Eu4GFND153bZubmUu4Rj59QaJTg3chpQDD8pun1DrpWs");

pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;

use instructions::*;

#[program]
pub mod prediction_market {
    use super::*;

    pub fn create_market(
        ctx: Context<CreateMarket>,
        created_at: i64,
        close_time: i64,
        question: String,
        category: String,
    ) -> Result<()> {
        instructions::create_market::create_market(ctx, created_at, close_time, question, category)
    }

    pub fn place_bet(ctx: Context<PlaceBet>, amount: u64, outcome: bool) -> Result<()> {
        instructions::place_bet::place_bet(ctx, amount, outcome)
    }

    pub fn resolve_market(ctx: Context<ResolveMarket>, outcome: bool) -> Result<()> {
        instructions::resolve_market::resolve_market(ctx, outcome)
    }

    pub fn claim_winnings(ctx: Context<ClaimWinnings>) -> Result<()> {
        instructions::claim_winnings::claim_winnings(ctx)
    }
}