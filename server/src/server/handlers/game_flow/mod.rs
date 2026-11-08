//! Match lifecycle: start requests, warmup expiry, early end, the match
//! timer, the scheduled return to the lobby, and auto-fill bot selection.

mod bot_fill;
mod match_end;
mod match_start;
mod start_request;

pub use match_end::{check_early_end_conditions, execute_scheduled_match_end, update_match_timer};
pub use match_start::maybe_start_match;
pub use start_request::handle_start_game;
