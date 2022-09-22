#![warn(clippy::suboptimal_flops)]

//! zen-jpeg, a JPEG decoder

pub use crate::decoder::Decoder;

mod decoder;

mod bitstream;
mod dct;
mod ec;
pub mod error;
