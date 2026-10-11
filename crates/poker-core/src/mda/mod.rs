//! Separate, local population datasets. No Hero statistics or identity targeting.
pub mod engine;
pub mod facts;
pub mod storage;
pub mod tools;
pub use engine::{Manager, Request};
