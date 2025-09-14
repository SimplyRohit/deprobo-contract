use anchor_lang::prelude::*;

#[event]
pub struct MarketCreated {
    pub market: Pubkey,
    pub created_at: i64,
    pub close_time: i64,
    pub authority: Pubkey,
    pub question: String,
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
    pub fee: u64,
}

#[event]
pub struct WinningsClaimed {
    pub market: Pubkey,
    pub user: Pubkey,
    pub payout: u64,
}