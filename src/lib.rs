//! zen-jpeg, a JPEG decoder

pub use crate::decoder::Decoder;

mod decoder;

mod bitstream;
mod ec;
pub mod error;
