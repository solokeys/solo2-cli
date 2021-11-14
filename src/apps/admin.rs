use lpc55::secure_binary::Version;

use super::App as _;
use crate::{Card, Result, Uuid};

pub struct App {
    pub card: Card,
}

impl super::App for App {
    const RID: &'static [u8] = super::SOLOKEYS_RID;
    const PIX: &'static [u8] = super::ADMIN_PIX;

    fn new(uuid: Option<Uuid>) -> Result<Self> {
        Ok(Self {
            card: Self::connect(uuid)?,
        })
    }

    fn card(&mut self) -> &mut Card {
        &mut self.card
    }
}

// TODO: Make new release of lpc55-host to reuse the code there.

//#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
///// The decoded version.
/////
///// Due to properties of the bootloader's internal 32-bit version,
///// we have 10 bits for major, 16 bits for minor, and 6 bits for
///// the patch component.
//pub struct Version {
//    pub major: u16,
//    pub minor: u16,
//    pub patch: u16,
//}

// impl Version {
//     pub fn minor_as_date(&self) -> NaiveDate {
//         use chrono::Duration;
//         let epoch = NaiveDate::from_ymd(2020, 1, 1);
//         let date = epoch + Duration::days(self.minor as _);
//         date
//     }
// }

// impl From<Version> for u32 {
//     fn from(version: Version) -> Self {
//         ((version.major as u32) << 22) | ((version.minor as u32) << 6) | (version.patch as u32)
//     }
// }

// impl fmt::Display for Version {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         // write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
//         write!(f, "{}:{}", self.major, self
//             .minor_as_date()
//             .format("%Y%m%d")
//             )?;
//         if self.patch > 0 {
//             write!(f, ".{}", self.patch)
//         } else {
//             Ok(())
//         }
//     }
// }

// impl From<[u8; 4]> for Version {
//     fn from(bytes: [u8; 4]) -> Self {
//         let version = u32::from_be_bytes(bytes);
//         let major = (version >> 22) as _;
//         let minor = ((version >> 6) & ((1 << 16) - 1)) as _;
//         let patch = (version & ((1 << 6) - 1)) as _;

//         Self {
//             major,
//             minor,
//             patch,
//         }
//     }
// }

impl App {
    pub const BOOT_TO_BOOTROM_COMMAND: u8 = 0x51;
    pub const REBOOT_COMMAND: u8 = 0x53;
    pub const VERSION_COMMAND: u8 = 0x61;
    pub const UUID_COMMAND: u8 = 0x62;

    pub fn boot_to_bootrom(&mut self) -> Result<()> {
        println!("Tap button on key...");
        // Rebooting can cause the connection to return error, which is ok here.
        self.call(Self::BOOT_TO_BOOTROM_COMMAND).map(drop).ok();
        Ok(())
    }

    pub fn reboot(&mut self) -> Result<()> {
        // Rebooting can cause the connection to return error, which is ok here.
        self.call(Self::REBOOT_COMMAND).map(drop).ok();
        Ok(())
    }

    pub fn uuid(&mut self) -> Result<u128> {
        let version_bytes = self.call(Self::UUID_COMMAND)?;
        let bytes: &[u8] = &version_bytes;
        bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("expected 16 byte UUID, got {}", &hex::encode(bytes)))
            .map(u128::from_be_bytes)
    }

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
