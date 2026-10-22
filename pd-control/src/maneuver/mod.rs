//! Experimental episode-local maneuver contract. The caller owns native time
//! and termination; implementations own admission, command authority and state.
mod contract;
mod landing;

pub use contract::*;
pub use landing::TerminalLandingController;

#[cfg(test)]
mod tests;
