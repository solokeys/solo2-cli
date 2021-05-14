// use hex_literal::hex;
// use iso7816::Instruction;

use crate::{Card, Result};

pub struct App {
    card: Card,
}

impl super::App for App {
    const RID: &'static [u8] = super::SOLOKEYS_RID;
    const PIX: &'static [u8] = super::TESTER_PIX;

    fn new() -> Result<Self> {
        Ok(Self {
            card: Self::connect()?,
        })
    }

    fn card(&mut self) -> &mut Card {
        &mut self.card
    }
}
