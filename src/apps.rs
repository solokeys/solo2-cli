//! Middleware to use the Trussed apps on a Solo 2 device.

use hex_literal::hex;

use crate::{Result, Transport};

/// Temporarily wrap an exclusive pointer to a transport, after selecting the app.
///
/// If instead apps were traits on transports - where would we store the app ID?
#[macro_export]
macro_rules! app(
    () => {

        pub struct App<'t> {
            #[allow(dead_code)]
            transport: &'t mut dyn $crate::Transport,
        }

        impl<'t> From<&'t mut dyn $crate::Transport> for App<'t> {
            fn from(transport: &'t mut dyn $crate::Transport) -> App<'t> {
                Self { transport }
            }
        }
    }
);

pub mod admin;
pub use admin::App as Admin;
pub mod ndef;
pub use ndef::App as Ndef;
pub mod oath;
pub use oath::App as Oath;
pub mod piv;
pub use piv::App as Piv;
pub mod provision;
pub mod qa;

/// well-known Registered Application Provider Identifiers.
pub struct Rid;
impl Rid {
    pub const NFC_FORUM: &'static [u8] = &hex!("D276000085");
    pub const NIST: &'static [u8] = &hex!("A000000308");
    pub const SOLOKEYS: &'static [u8] = &hex!("A000000847");
    pub const YUBICO: &'static [u8] = &hex!("A000000527");
}

/// well-known Proprietary Application Identifier Extensions.
pub struct Pix;
impl Pix {
    pub const ADMIN: &'static [u8] = &hex!("00000001");
    pub const NDEF: &'static [u8] = &hex!("0101");
    pub const OATH: &'static [u8] = &hex!("2101");
    // the full PIX ends with 0100 for version 01.00,
    // truncated is enough to select
    // pub const PIV_VERSIONED: &'static [u8] = &hex!("000010000100");
    pub const PIV: &'static [u8] = &hex!("00001000");
    pub const PROVISION: &'static [u8] = &hex!("01000001");
    pub const QA: &'static [u8] = &hex!("01000000");
}

pub trait Select<'t>: From<&'t mut dyn Transport> {
    const RID: &'static [u8];
    const PIX: &'static [u8];

    fn application_id() -> Vec<u8> {
        let mut aid: Vec<u8> = Default::default();
        aid.extend_from_slice(Self::RID);
        aid.extend_from_slice(Self::PIX);
        aid
    }

    fn select(transport: &'t mut dyn Transport) -> Result<Self> {
        transport.select(Self::application_id())?;
        Ok(Self::from(transport))
    }
}
