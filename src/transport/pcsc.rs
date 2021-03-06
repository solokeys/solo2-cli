use anyhow::anyhow;
use iso7816::Status;

pub use crate::{device::pcsc::Device, Error, Result};

impl Device {
    pub fn call(
        &mut self,
        cla: u8,
        ins: u8,
        p1: u8,
        p2: u8,
        data: Option<&[u8]>,
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

        let l = self.device.transmit(&send_buffer, &mut recv_buffer)?.len();
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
