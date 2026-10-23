//! Experimental episode-local maneuver contract. The caller owns native time
//! and termination; implementations own admission, command authority and state.
mod arrival_landing;
mod coast_landing;
mod contract;
mod landing;
mod poly_landing;
mod timed;

pub use arrival_landing::{ArrivalLandingController, ArrivalLandingProgram, LandingSearchFeedback};
pub use coast_landing::CoastLandingController;
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
