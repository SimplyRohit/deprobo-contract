use anchor_lang::prelude::*;

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

    #[msg("Bet amount is invalid")]
    BetAmountInvalid,

    #[msg("Market mismatch for this bet")]
    MarketMismatch,
}