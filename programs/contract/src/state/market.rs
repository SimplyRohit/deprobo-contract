use anchor_lang::prelude::*;

#[account]
pub struct Market {
    pub authority: Pubkey,
    pub bet: bool,
    pub created_at: i64,
    pub close_time: i64,
    pub yes_pool: Pubkey,
    pub no_pool: Pubkey,
    pub total_yes: u64,
    pub total_no: u64,
    pub resolved: bool,
    pub winning_outcome: bool,
    pub fee_collected: u64,
}

impl Market {
    pub const SIZE: usize = 32 + 1 + 8 + 8 + 32 + 32 + 8 + 8 + 1 + 1 + 8;
}