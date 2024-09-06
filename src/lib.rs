//#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![warn(clippy::allow_attributes)]
#![warn(clippy::allow_attributes_without_reason)]
#![warn(clippy::dbg_macro)]
#![warn(clippy::expect_used)]
#![warn(clippy::pedantic)]
#![warn(clippy::unwrap_used)]
#![warn(rust_2018_idioms, unused_lifetimes, missing_debug_implementations)]

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
