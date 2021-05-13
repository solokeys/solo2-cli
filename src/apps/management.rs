use core::convert::TryInto;

use crate::{Card, Result};
use super::App as _;

pub struct App {
    card: Card,
}

impl super::App for App {
    const RID: &'static [u8] = super::SOLOKEYS_RID;
    const PIX: &'static [u8] = super::MANAGEMENT_PIX;

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
    const BOOT_TO_BOOTROM_COMMAND: u8 = 0x51;
    const REBOOT_COMMAND: u8 = 0x53;
    const VERSION_COMMAND: u8 = 0x61;
    const UUID_COMMAND: u8 = 0x62;

    fn call(&mut self, command: u8) -> Result<Vec<u8>> {
        self.card.call(
            0,
            command,
            0x00,
            0x00,
            Some(&Self::aid()),
        )
    }
    pub fn boot_to_bootrom(&mut self) -> Result<()> {
        self.call(Self::BOOT_TO_BOOTROM_COMMAND).map(drop)
    }

    pub fn reboot(&mut self) -> Result<()> {
        self.call(Self::REBOOT_COMMAND).map(drop)
    }

    pub fn uuid(&mut self) -> Result<u128> {
        let version_bytes = self.call(Self::UUID_COMMAND)?;
        let bytes: &[u8] = &version_bytes;
        bytes.try_into()
            .map_err(|_| anyhow::anyhow!("expected 16 byte UUID, got {}", &hex::encode(bytes)))
            .map(|bytes| u128::from_be_bytes(bytes))
    }

    pub fn version(&mut self) -> Result<[u8; 4]> {
        let version_bytes = self.call(Self::VERSION_COMMAND)?;
        let bytes: &[u8] = &version_bytes;
        bytes.try_into()
            .map_err(|_| anyhow::anyhow!("expected 4 bytes version, got {}", &hex::encode(bytes)))
    }
}
