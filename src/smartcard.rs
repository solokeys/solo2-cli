use anyhow::anyhow;
use core::convert::TryInto;
use core::convert::TryFrom;
use iso7816::Status;

use pcsc::{Context, Protocols, Scope, ShareMode};

use crate::apps;

#[derive(Copy, Clone, Debug)]
pub enum Filter {
    AllCards,
    SoloCards,
}

impl Default for Filter {
    fn default() -> Self { Filter::AllCards }
}

pub struct Card {
    card: pcsc::Card,
    pub reader_name: String,
    pub uuid: Option<u128>,
}

impl TryFrom<(&std::ffi::CStr, &Context)> for Card {
    type Error = anyhow::Error;
    fn try_from(pair: (&std::ffi::CStr, &Context)) -> crate::Result<Self> {
        let (reader, context) = pair;
        let mut card = context.connect(reader, ShareMode::Shared, Protocols::ANY)?;
        let uuid_maybe = Self::try_reading_uuid(&mut card)
            .map(|uuid| u128::from_be_bytes(uuid)).ok();
        Ok(Self { card, reader_name: reader.to_str().unwrap().to_owned(), uuid: uuid_maybe })
    }
}

impl Card {

    pub fn list(filter: Filter) -> Vec<Self> {
        let cards = match Self::list_failable() {
            Ok(cards) => {
                cards
            }
            _ => {
                Default::default()
            }
        };
        match filter {
            Filter::AllCards => {
                cards
            },
            Filter::SoloCards => {
                cards.into_iter().filter(|card| card.uuid.is_some()).collect()
            }
        }
    }

    pub fn list_failable() -> crate::Result<Vec<Self>> {

        let mut cards_with_trussed: Vec<Self> = Default::default();

        let context = Context::establish(Scope::User)?;
        let l = context.list_readers_len()?;
        let mut buffer = Vec::with_capacity(l);
        buffer.resize(l, 0);

        let readers = context.list_readers(&mut buffer)?.collect::<Vec<_>>();

        for reader in readers {
            info!("connecting with reader: `{}`", &reader.to_string_lossy());
            let card_maybe = Self::try_from((reader, &context));
            info!("...connected");

            match card_maybe {
                Ok(card) => {
                    cards_with_trussed.push(card);
                    debug!("Reader has a card.");
                },
                Err(_err) => {
                    // Not a Trussed supported device.
                    info!("could not connect to card on reader, skipping ({:?}).", _err);
                }
            }
        }
        Ok(cards_with_trussed)
    }

    // Try to read Solo2 uuid
    fn try_reading_uuid(card: &mut pcsc::Card) -> crate::Result<[u8; 16]> {
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

        let uuid_bytes = Self::call_card(
            card,
            0,
            apps::admin::App::UUID_COMMAND,
            0x00,
            0x00,
            None,
        )?;
        
        if uuid_bytes.len() == 16 {
            let mut uuid = [0u8; 16];
            uuid.clone_from_slice(&uuid_bytes);

            Ok(uuid)
        } else {
            Err(anyhow!("Did not read 16 byte uuid from mgmt app."))
        }
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
    ) -> crate::Result<Vec<u8>> {


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
        if l <= 255 {
            // Le = 256
            send_buffer.push(0);
        } else {
            send_buffer.push(0);
            send_buffer.push(0);
        }

        debug!(">> {}", hex::encode(&send_buffer));

        let mut recv_buffer = Vec::<u8>::with_capacity(3072);
        recv_buffer.resize(3072, 0);

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
            return Err(if recv_buffer.len() > 0 {
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
    ) -> crate::Result<Vec<u8>> {
        Self::call_card(&mut self.card, cla, ins, p1, p2, data)
    }
}
