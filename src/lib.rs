//#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::unwrap_used)]
#![warn(rust_2018_idioms, unused_lifetimes, missing_debug_implementations)]

pub mod docker;
pub mod image_name;
pub mod manifest;

pub use docker::{
    Client,
    Error as ClientError,
    Response,
};
pub use image_name::{
    ImageName,
    Registry,
    Tag,
};
pub use manifest::Manifest;
