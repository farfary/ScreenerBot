//! Exit strategy coordination

mod roi;
mod time_override;
mod trailing_stop;

pub use roi::check_roi_exit;
pub use time_override::check_time_override;
pub use trailing_stop::check_trailing_stop;
