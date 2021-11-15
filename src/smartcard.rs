//! Smartcard interface to Solo 2 devices.
//!
//! Might grow into an independent more idiomatic replacement for [pcsc][pcsc],
//! which is currently used internally.
//!
//! A long-shot idea is to rewrite the entire PCSC/CCID stack in pure Rust,
//! restricting to "things that are directly attached via USB", i.e. ICCD.
//!
//! Having that would allow doing the essentially same thing over HID (instead of
//! reinventing a custom HID class), or even a custom USB class (circumventing
//! the WebUSB/WebHID restrictions). Also we could easily upgrade to USB 2.0 HS,
//! instead of being stuck with full-speed USB only.
//!
//! [pcsc]: https://docs.rs/pcsc/
//!

use anyhow::anyhow;
use core::fmt;
use iso7816::Status;
use lpc55::bootloader::UuidSelectable;

use pcsc::{Protocols, Scope, ShareMode};

use crate::{apps, Result, Uuid};

/// The PCSC service (running `pcscd` instance)
pub struct Service {
    session: pcsc::Context,
}

impl Service {
    /// The environment may not have an accessible PCSC service running.
    ///
    /// This performs the check.
    pub fn is_available() -> bool {
        Self::user_session().is_ok()
    }

    /// Establishes a user session with the PCSC service, if available.
    pub fn user_session() -> Result<Self> {
        Ok(Self { session:  pcsc::Context::establish(Scope::User)? })
    }

    /// Get a connection to a smartcard by name.
    ///
    /// We prefer to use normal Rust types for the smartcard names, meaning
    /// that cards with weird non-UTF8 names won't be addressable.
    pub fn connect(&self, smartcard_name: &str) -> Result<Smartcard> {
        let cstring = std::ffi::CString::new(smartcard_name.as_bytes()).unwrap();
        Ok(Smartcard {
            card: self.session.connect(&cstring, ShareMode::Shared, Protocols::ANY)?,
            name: smartcard_name.to_string(),
        })
    }

    /// List all smartcards names in the system.
    ///
    /// We prefer to use normal Rust types for the smartcard names, meaning
    /// that cards with weird non-UTF8 names won't be addressable.
    pub fn smartcard_names(&self) -> Result<Vec<String>> {
        let mut card_names_buffer = vec![0; self.session.list_readers_len()?];
        let names = self.session.list_readers(&mut card_names_buffer)?
            .map(|name_cstr| name_cstr.to_string_lossy().to_string())
            .collect();
        Ok(names)
    }

    /// Get all of the usable smartcards in the system.
    pub fn smartcards(&self) -> Result<Vec<Smartcard>> {
        Ok(self.smartcard_names()?
           .iter()
           .filter_map(|name| self.connect(name).ok())
           .collect()
       )
    }
}

/// An [ICCD][iccd] smartcard.
///
/// This object does not necessarily have a UUID, which is a Trussed/SoloKeys thing.
///
/// [iccd]: https://www.usb.org/document-library/smart-card-iccd-version-10
pub struct Smartcard {
    card: pcsc::Card,
    pub name: String,
}

impl fmt::Debug for Smartcard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::result::Result<(), fmt::Error> {
        write!(f, "{}", &self.name)
    }
}

impl fmt::Display for Smartcard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::result::Result<(), fmt::Error> {
        fmt::Debug::fmt(self, f)
    }
}

impl UuidSelectable for Smartcard {
    fn try_uuid(&mut self) -> Result<Uuid> {

        let mut aid: Vec<u8> = Default::default();
        aid.extend_from_slice(apps::SOLOKEYS_RID);
        aid.extend_from_slice(apps::ADMIN_PIX);

        self.call(
            0, iso7816::Instruction::Select.into(),
            0x04, 0x00,
            Some(&aid),
        )?;

        let uuid_bytes: [u8; 16] = self.call(
            0, apps::admin::App::UUID_COMMAND,
            0x00, 0x00,
            None,
        )?
            .try_into()
            .map_err(|_| anyhow!("Did not read 16 byte uuid from mgmt app."))?;

        let uuid = Uuid::from_bytes(uuid_bytes);
        Ok(uuid)
    }

    /// Infallible method listing all usable smartcards.
    ///
    /// To find out more about issues along the way, construct the session,
    /// list the smartcards, attach them, etc.
    fn list() -> Vec<Self> {
        let session = match Service::user_session() {
            Ok(session) => session,
            _ => return Vec::default(),
        };
        session.smartcards().unwrap_or_else(|_| Vec::default())
    }

    fn having(uuid: Uuid) -> Result<Self> {
        use super::Device;
        let device = Device::having(uuid)?;
        match device {
            Device::Solo2(solo2) => Ok(solo2.into_inner()),
            _ => Err(anyhow!("No smartcard found with UUID {:X}", uuid.to_simple())),
        }
    }
}

impl Smartcard {
    pub fn call(&mut self, cla: u8, ins: u8, p1: u8, p2: u8, data: Option<&[u8]>) -> Result<Vec<u8>> {
        let data = data.unwrap_or(&[]);
        let mut send_buffer = Vec::<u8>::with_capacity(data.len() + 16);

        send_buffer.push(cla);
        send_buffer.push(ins);
        send_buffer.push(p1);
        send_buffer.push(p2);

        // TODO: checks, chain, ...
        let l = data.len();
        if l > 0 {
            if l <= 255 {
                send_buffer.push(l as u8);
            } else {
                send_buffer.push(0);
                send_buffer.extend_from_slice(&(l as u16).to_be_bytes());
            }
            send_buffer.extend_from_slice(data);
        }

        send_buffer.push(0);
        if l > 255 {
            send_buffer.push(0);
        }

        debug!(">> {}", hex::encode(&send_buffer));

        let mut recv_buffer = vec![0; 3072];

        let l = self.card.transmit(&send_buffer, &mut recv_buffer)?.len();
        debug!("RECV {} bytes", l);
        recv_buffer.resize(l, 0);
        debug!("<< {}", hex::encode(&recv_buffer));

        if l < 2 {
            return Err(anyhow!(
                "response should end with two status bytes! received {}",
                hex::encode(recv_buffer)
            ));
        }
        let sw2 = recv_buffer.pop().unwrap();
        let sw1 = recv_buffer.pop().unwrap();

        let status = (sw1, sw2).try_into();
        if Ok(Status::Success) != status {
            return Err(if !recv_buffer.is_empty() {
                anyhow!(
                    "card signaled error {:?} ({:X}, {:X}) with data {}",
                    status,
                    sw1,
                    sw2,
                    hex::encode(recv_buffer)
                )
            } else {
                anyhow!("card signaled error: {:?} ({:X}, {:X})", status, sw1, sw2)
            });
        }

        Ok(recv_buffer)
    }
}

