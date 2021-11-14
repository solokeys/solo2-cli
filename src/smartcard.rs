use anyhow::anyhow;
use core::fmt;
use iso7816::Status;

use pcsc::{Protocols, Scope, ShareMode};

use crate::{apps, Result, Uuid};

#[derive(Copy, Clone, Debug)]
pub enum Filter {
    AllCards,
    SoloCards,
}

impl Default for Filter {
    fn default() -> Self {
        Filter::AllCards
    }
}

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

pub struct Smartcard {
    card: pcsc::Card,
    pub name: String,
}

impl fmt::Debug for Smartcard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::result::Result<(), fmt::Error> {
        write!(f, "{}", &self.name)
    }
}

impl Smartcard {
    /// Infallible method listing all usable smartcards.
    ///
    /// To find out more about issues along the way, construct the session,
    /// list the smartcards, attach them, etc.
    pub fn list() -> Vec<Self> {
        let session = match Service::user_session() {
            Ok(session) => session,
            _ => return Vec::default(),
        };
        session.smartcards().unwrap_or_else(|_| Vec::default())
    }

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

// #[deprecated]
pub struct Card {
    card: pcsc::Card,
    pub reader_name: String,
    pub uuid: Option<Uuid>,
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(uuid) = self.uuid {
            // format!("\"{}\" UUID: {}", card.reader_name, hex::encode(uuid.to_be_bytes()))
            f.write_fmt(format_args!("Solo 2 {:X}", uuid))
        } else {
            f.write_fmt(format_args!(" \"{}\"", self.reader_name))
        }
    }
}

impl TryFrom<(&std::ffi::CStr, &pcsc::Context)> for Card {
    type Error = anyhow::Error;
    fn try_from(pair: (&std::ffi::CStr, &pcsc::Context)) -> Result<Self> {
        let (reader, context) = pair;
        let mut card = context.connect(reader, ShareMode::Shared, Protocols::ANY)?;
        let uuid_maybe = Self::try_reading_uuid(&mut card).ok();
        Ok(Self {
            card,
            reader_name: reader.to_str().unwrap().to_owned(),
            uuid: uuid_maybe,
        })
    }
}

impl Card {
    pub fn list(filter: Filter) -> Vec<Self> {
        let cards = match Self::try_list() {
            Ok(cards) => cards,
            _ => Default::default(),
        };
        match filter {
            Filter::AllCards => cards,
            Filter::SoloCards => cards
                .into_iter()
                .filter(|card| card.uuid.is_some())
                .collect(),
        }
    }

    pub fn try_list() -> Result<Vec<Self>> {
        let mut cards_with_trussed: Vec<Self> = Default::default();

        let context = pcsc::Context::establish(Scope::User)?;
        let l = context.list_readers_len()?;
        let mut buffer = vec![0; l];

        let readers = context.list_readers(&mut buffer)?.collect::<Vec<_>>();

        for reader in readers {
            info!("connecting with reader: `{}`", &reader.to_string_lossy());
            let card_maybe = Self::try_from((reader, &context));
            info!("...connected");

            match card_maybe {
                Ok(card) => {
                    cards_with_trussed.push(card);
                    debug!("Reader has a card.");
                }
                Err(_err) => {
                    // Not a Trussed supported device.
                    info!(
                        "could not connect to card on reader, skipping ({:?}).",
                        _err
                    );
                }
            }
        }
        Ok(cards_with_trussed)
    }

    // Try to read Solo2 uuid
    fn try_reading_uuid(card: &mut pcsc::Card) -> Result<Uuid> {
        let mut aid: Vec<u8> = Default::default();
        aid.extend_from_slice(apps::SOLOKEYS_RID);
        aid.extend_from_slice(apps::ADMIN_PIX);

        Self::call_card(
            card,
            // Class::
            0,
            iso7816::Instruction::Select.into(),
            0x04,
            0x00,
            Some(&aid),
        )?;

        let uuid_bytes =
            Self::call_card(card, 0, apps::admin::App::UUID_COMMAND, 0x00, 0x00, None)?;

        Ok(Uuid::from_bytes(
            uuid_bytes.try_into()
            .map_err(|_| anyhow!("Did not read 16 byte uuid from mgmt app."))?
        ))
    }

    fn call_card(
        card: &mut pcsc::Card,
        // cla: Into<u8>, ins: Into<u8>,
        // p1: Into<u8>, p2: Into<u8>,
        cla: u8,
        ins: u8,
        p1: u8,
        p2: u8,
        data: Option<&[u8]>,
        // ) -> iso7816::Result<Vec<u8>> {
    ) -> Result<Vec<u8>> {
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

        let l = card.transmit(&send_buffer, &mut recv_buffer)?.len();
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

    pub fn call(
        &mut self,
        // cla: Into<u8>, ins: Into<u8>,
        // p1: Into<u8>, p2: Into<u8>,
        cla: u8,
        ins: u8,
        p1: u8,
        p2: u8,
        data: Option<&[u8]>,
        // ) -> iso7816::Result<Vec<u8>> {
    ) -> Result<Vec<u8>> {
        Self::call_card(&mut self.card, cla, ins, p1, p2, data)
    }
}
