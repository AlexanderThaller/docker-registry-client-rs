//#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(clippy::unwrap_used)]
#![warn(rust_2018_idioms, unused_lifetimes, missing_debug_implementations)]
#![warn(clippy::dbg_macro)]

pub mod docker;
pub mod image;
pub mod manifest;

pub use docker::{
    Client,
    Error as ClientError,
    Response,
};
pub use image::{
    image_name::{
        digest::Digest,
        tag::Tag,
        ImageName,
    },
    registry::Registry,
    Image,
};
pub use manifest::Manifest;
