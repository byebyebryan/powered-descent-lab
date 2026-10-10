//! Strategy-independent evaluation, immutable capture and authoritative replay.
//! Built-in registration and legacy workflows live in `pd-eval-cli`.

mod capture;
mod collection;
mod episode;
mod evidence;
pub use capture::*;
pub use collection::*;
pub use episode::*;
pub use evidence::*;
