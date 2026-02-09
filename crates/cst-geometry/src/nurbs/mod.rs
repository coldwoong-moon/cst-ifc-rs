//! NURBS core algorithms: knot vector utilities and De Boor evaluation.

pub mod deboor;
pub mod knot;

pub use deboor::*;
pub use knot::{basis_functions, basis_functions_derivs, find_span};
