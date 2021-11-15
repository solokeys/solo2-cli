use lpc55::secure_binary::Version;

use super::App as _;
use crate::{Smartcard, Result, Uuid};

app_boilerplate!();

impl super::App for App {
    const RID: &'static [u8] = super::SOLOKEYS_RID;
    const PIX: &'static [u8] = super::ADMIN_PIX;
}

impl App {
    pub const BOOT_TO_BOOTROM_COMMAND: u8 = 0x51;
    pub const REBOOT_COMMAND: u8 = 0x53;
    pub const VERSION_COMMAND: u8 = 0x61;
    pub const UUID_COMMAND: u8 = 0x62;

    /// Reboot the Solo 2 to bootloader mode.
    ///
    /// Rebooting can cause the connection to return error, which should
    /// be special-cased by the caller.
    pub fn boot_to_bootrom(&mut self) -> Result<()> {
        self.call(Self::BOOT_TO_BOOTROM_COMMAND).map(drop)
    }

    /// Reboot the Solo 2 normally.
    ///
    /// Rebooting can cause the connection to return error, which should
    /// be special-cased by the caller.
    pub fn reboot(&mut self) -> Result<()> {
        self.call(Self::REBOOT_COMMAND).map(drop)
    }

    /// The UUID of the device.
    ///
    /// This can be fetched in multiple other ways, and is also visible in bootloader mode.
    /// Responding successfully to this command is our criterion for treating a smartcard
    /// as a Solo 2 device.
    pub fn uuid(&mut self) -> Result<Uuid> {
        let version_bytes = self.call(Self::UUID_COMMAND)?;
        let bytes: &[u8] = &version_bytes;
        Ok(Uuid::from_u128(bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("expected 16 byte UUID, got {}", &hex::encode(bytes)))
            .map(u128::from_be_bytes)?))
    }

    /// The version of the [Firmware][crate::Firmware] currently running on the Solo 2.
    pub fn version(&mut self) -> Result<Version> {
        let version_bytes = self.call(Self::VERSION_COMMAND)?;
        let bytes: [u8; 4] = version_bytes.as_slice().try_into().map_err(|_| {
            anyhow::anyhow!(
                "expected 4 bytes version, got {}",
                &hex::encode(version_bytes)
            )
        })?;
        Ok(bytes.into())
    }
}
