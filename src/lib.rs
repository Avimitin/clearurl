#![cfg_attr(
    debug_assertions,
    allow(dead_code, unused_imports, unused_variables, unused_mut)
)]

pub mod data;
pub mod filter;

pub use filter::clear;
