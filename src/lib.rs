#![deny(warnings, trivial_casts, unused_qualifications)]

#[macro_use]
extern crate log;

pub mod apps;
pub mod device;
pub use device::{Device, Solo2};
#[cfg(feature = "dev-pki")]
pub mod dev_pki;
pub mod error;
pub use error::{Error, Result};
pub mod firmware;
pub use firmware::{Firmware, Version};
pub mod smartcard;
pub use smartcard::Smartcard;
pub use lpc55::{UuidSelectable, uuid::Uuid};
