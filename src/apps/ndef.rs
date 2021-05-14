use hex_literal::hex;
use iso7816::Instruction;

use crate::{Card, Result};

pub struct App {
    card: Card,
}

impl super::App for App {
    const RID: &'static [u8] = super::NFC_FORUM_RID;
    const PIX: &'static [u8] = super::NDEF_PIX;

    fn new() -> Result<Self> {
        Ok(Self {
            card: Self::connect()?,
        })
    }

    fn card(&mut self) -> &mut Card {
        &mut self.card
    }
}

impl App {
    const CAPABILITIES_PARAMETER: [u8; 2] = hex!("E103");
    const DATA_PARAMETER: [u8; 2] = hex!("E104");

    fn call_with(&mut self, command: u8, data: [u8; 2]) -> Result<Vec<u8>> {
        self.card.call(0, command, 0x00, 0x00, Some(&data))
    }

    fn fetch(&mut self) -> Result<Vec<u8>> {
        self.card
            .call(0, Instruction::ReadBinary.into(), 0x00, 0x00, None)
    }

    pub fn capabilities(&mut self) -> Result<Vec<u8>> {
        self.call_with(Instruction::Select.into(), Self::CAPABILITIES_PARAMETER)
            .map(drop)?;
        self.fetch()
    }

    pub fn data(&mut self) -> Result<Vec<u8>> {
        self.call_with(Instruction::Select.into(), Self::DATA_PARAMETER)
            .map(drop)?;
        self.fetch()
    }
}
