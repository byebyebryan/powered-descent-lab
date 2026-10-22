//! Poly Landing shared finite-reference construction and native actuation.
//! Coast, duration selection, route planning and episode execution stay outside.
mod actuation;
mod reference;

pub use actuation::paired_throttle;
pub use reference::{LandingKinematicsV1, PolynomialReferenceV1, build_reference, paired_mean};

#[cfg(test)]
mod tests;
