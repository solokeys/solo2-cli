use iso7816::Instruction;

use super::App as _;
use crate::{Smartcard, Result};

app_boilerplate!();

impl super::App for App {
    const RID: &'static [u8] = super::NFC_FORUM_RID;
    const PIX: &'static [u8] = super::NDEF_PIX;
}

impl App {
    const CAPABILITIES_PARAMETER: [u8; 2] = [0xE1, 0x03];
    const DATA_PARAMETER: [u8; 2] = [0xE1, 0x04];

    fn fetch(&mut self) -> Result<Vec<u8>> {
        self.card
            .call(0, Instruction::ReadBinary.into(), 0x00, 0x00, None)
    }

    pub fn capabilities(&mut self) -> Result<Vec<u8>> {
        self.call_with(Instruction::Select.into(), &Self::CAPABILITIES_PARAMETER)
            .map(drop)?;
        self.fetch()
    }

    pub fn data(&mut self) -> Result<Vec<u8>> {
        self.call_with(Instruction::Select.into(), &Self::DATA_PARAMETER)
            .map(drop)?;
        self.fetch()
    }
}
