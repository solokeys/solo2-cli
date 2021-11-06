// use hex_literal::hex;
// use iso7816::Instruction;

use crate::{Card, Result, Uuid};

pub struct App {
    pub card: Card,
}

impl super::App for App {
    const RID: &'static [u8] = super::SOLOKEYS_RID;
    const PIX: &'static [u8] = super::TESTER_PIX;

    fn new(uuid: Option<Uuid>) -> Result<Self> {
        Ok(Self {
            card: Self::connect(uuid)?,
        })
    }

    fn card(&mut self) -> &mut Card {
        &mut self.card
    }
}
