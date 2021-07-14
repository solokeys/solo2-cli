use anyhow::anyhow;
use core::convert::TryInto;
use iso7816::Status;

use crate::apps;

pub struct Card {
    card: pcsc::Card,
    uuid: Option<[u8; 16]>
}

impl From<pcsc::Card> for Card {
    fn from(card: pcsc::Card) -> Self {
        Self { card, uuid: None }
    }
}

impl Card {

    // Try to read Solo2 uuid
    pub fn try_reading_uuid(&mut self) -> crate::Result<[u8; 16]> {
        let mut aid: Vec<u8> = Default::default();
        aid.extend_from_slice(apps::SOLOKEYS_RID);
        aid.extend_from_slice(apps::ADMIN_PIX);

        self.call(
            // Class::
            0,
            iso7816::Instruction::Select.into(),
            0x04,
            0x00,
            Some(&aid),
        )?;

        let uuid_bytes = self.call(
            0,
            apps::admin::App::UUID_COMMAND,
            0x00,
            0x00,
            None,
        )?;
        
        if uuid_bytes.len() == 16 {
            let mut uuid = [0u8; 16];
            uuid.clone_from_slice(&uuid_bytes);
            self.uuid = Some(uuid.clone());

            Ok(uuid)
        } else {
            Err(anyhow!("Did not read 16 byte uuid from mgmt app."))
        }
    }

    pub fn last_read_uuid(&self) -> Option<[u8; 16]> {
        return self.uuid.clone();
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
}
