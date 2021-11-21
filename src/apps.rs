//! Middleware to use the Trussed apps on a Solo 2 device.

use hex_literal::hex;
use lpc55::UuidSelectable;

// use crate::device::{prompt_user_to_select_device, Device};
use crate::{Smartcard, Result, Uuid};

/// This feels totally boilerplatey, didn't think hard about how to reformulate yet.
///
/// For instance, the Apps might be extension traits, implemented on the Smartcard itself?
#[macro_export]
macro_rules! app_boilerplate(
    () => {

        pub struct App {
            pub card: $crate::Smartcard,
        }

        impl From<$crate::Smartcard> for App {
            fn from(card: $crate::Smartcard) -> Self {
                Self { card }
            }
        }

        impl From<App> for $crate::Smartcard {
            fn from(app: App) -> Self {
                app.card
            }
        }

        impl AsRef<$crate::Smartcard> for App {
            fn as_ref(&self) -> &$crate::Smartcard {
                &self.card
            }
        }

        impl AsMut<$crate::Smartcard> for App {
            fn as_mut(&mut self) -> &mut $crate::Smartcard {
                &mut self.card
            }
        }
    }
);

pub mod admin;
pub mod ndef;
pub mod oath;
// pub mod piv;
pub mod provisioner;
pub mod tester;

pub const NFC_FORUM_RID: &[u8] = &hex!("D276000085");
pub const NIST_RID: &[u8] = &hex!("A000000308");
pub const SOLOKEYS_RID: &[u8] = &hex!("A000000847");
pub const YUBICO_RID: &[u8] = &hex!("A000000527");

pub const ADMIN_PIX: &[u8] = &hex!("00000001");
pub const NDEF_PIX: &[u8] = &hex!("0101");
pub const OATH_PIX: &[u8] = &hex!("2101");
// the full PIX ends with 0100 for version 01.00,
// truncated is enough to select
// pub const PIV_PIX: &[u8] = &hex!("000010000100");
pub const PIV_PIX: &[u8] = &hex!("00001000");
pub const PROVISIONER_PIX: &[u8] = &hex!("01000001");
pub const TESTER_PIX: &[u8] = &hex!("01000000");

pub trait App: AsRef<Smartcard> + AsMut<Smartcard> + From<Smartcard> + Into<Smartcard> + Sized {
    const RID: &'static [u8];
    const PIX: &'static [u8];

    fn aid() -> Vec<u8> {
        let mut aid: Vec<u8> = Default::default();
        aid.extend_from_slice(Self::RID);
        aid.extend_from_slice(Self::PIX);
        aid
    }

    fn select(&mut self) -> Result<Vec<u8>> {
        // use iso7816::command::class::Class;
        info!("selecting app: {}", hex::encode(Self::aid()).to_uppercase());

        self.card().call(
            // Class::
            0,
            iso7816::Instruction::Select.into(),
            0x04,
            0x00,
            Some(&Self::aid()),
        )
    }

    fn card(&mut self) -> &mut Smartcard {
        self.as_mut()
    }

    fn new(uuid: Uuid) -> Result<Self> {
        let card = crate::Smartcard::having(uuid)?;
        Ok(Self::with(card))
    }

    fn with(card: Smartcard) -> Self {
        Self::from(card)
    }

    fn into_inner(self) -> Smartcard {
        self.into()
    }

    fn call(&mut self, instruction: u8) -> Result<Vec<u8>> {
        self.card().call(0, instruction, 0x00, 0x00, None)
    }

    fn call_with(&mut self, instruction: u8, data: &[u8]) -> Result<Vec<u8>> {
        self.card().call(0, instruction, 0x00, 0x00, Some(data))
    }

    fn print_aid() {
        println!("{}", hex::encode(Self::aid()).to_uppercase());
    }
}

