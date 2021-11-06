#![deny(warnings, trivial_casts, unused_qualifications)]

#[macro_use]
extern crate log;

pub mod apps;
pub mod device;
pub use device::Device;
#[cfg(feature = "dev-pki")]
pub mod dev_pki;
pub mod error;
pub use error::{Error, Result};
pub mod smartcard;
pub use smartcard::Card;
pub mod uuid;
pub use uuid::Uuid;
pub mod update;
