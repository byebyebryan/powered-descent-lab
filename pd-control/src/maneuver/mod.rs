//! Experimental episode-local maneuver contract. The caller owns native time
//! and termination; implementations own admission, command authority and state.
mod contract;
mod landing;
mod poly_landing;
mod timed;

pub use contract::*;
pub use landing::TerminalLandingController;
pub use poly_landing::{
    CHECKED_POLY_LANDING_BACKEND, POLY_LANDING_BACKEND, PolynomialLandingController,
};
pub use timed::*;

#[cfg(test)]
mod poly_landing_tests;
#[cfg(test)]
mod tests;
