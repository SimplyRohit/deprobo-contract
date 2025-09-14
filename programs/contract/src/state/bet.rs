use anchor_lang::prelude::*;

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