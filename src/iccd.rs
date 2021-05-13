pub struct Card {
    card: pcsc::Card,
}

impl From<pcsc::Card> for Card {
    fn from(card: pcsc::Card) -> Self {
        Self { card }
    }
}

impl Card {
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
        if data.len() > 0 {
            send_buffer.push(data.len() as u8);
            send_buffer.extend_from_slice(data);
        }

        let mut recv_buffer = Vec::<u8>::with_capacity(3072);
        recv_buffer.resize(3072, 0);

        let l = self.card.transmit(&send_buffer, &mut recv_buffer)?.len();
        recv_buffer.resize(l, 0);

        if l < 2 {
            return Err(anyhow::anyhow!("response should end with two status bytes! received {}", hex::encode(recv_buffer)));
        }
        let sw2 = recv_buffer.pop().unwrap();
        let sw1 = recv_buffer.pop().unwrap();

        if (sw1, sw2) != (0x90, 0x00) {
            return Err(
                if recv_buffer.len() > 0 {
                    anyhow::anyhow!("card signaled error ({:X}, {:X}) with data {}", sw1, sw2, hex::encode(recv_buffer))
                } else {
                    anyhow::anyhow!("card signaled error: ({:X}, {:X})", sw1, sw2)
                }
            );
        }

        Ok(recv_buffer)
    }
}
