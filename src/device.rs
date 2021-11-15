//! Solo 2 devices, which may be in regular or bootloader mode.
use anyhow::anyhow;
use lpc55::bootloader::{Bootloader, UuidSelectable};

use crate::{Error, Firmware, Result, Smartcard, Uuid};
use core::fmt;

/// A [SoloKeys][solokeys] [Solo 2][solo2] device, in regular mode.
///
/// From an inventory perspective, the core identifier is a UUID (16 bytes / 128 bits).
///
/// From an interface perspective, currently only the smartcard interface is exposed and used.
/// Soon we will add the CTAP interface, at least for rebooting into the bootloader/update mode.
///
/// [solokeys]: https://solokeys.com
/// [solo2]: https://solo2.dev
pub struct Solo2 {
    card: Smartcard,
    uuid: Uuid,
}

impl fmt::Debug for Solo2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::result::Result<(), fmt::Error> {
        write!(f, "Solo 2 {:X} ({})", &self.uuid.to_simple(), &self.card.name)
    }
}

impl fmt::Display for Solo2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Solo 2 {:X}", &self.uuid.to_simple())
    }
}

impl UuidSelectable for Solo2 {

    fn try_uuid(&mut self) -> Result<Uuid> {
        Ok(self.uuid)
    }

    fn list() -> Vec<Self> {
        let smartcards = Smartcard::list();
        smartcards.into_iter()
            .filter_map(|card| Self::try_from(card).ok())
            .collect()
    }
}

impl Solo2 {
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn into_inner(self) -> Smartcard {
        self.card
    }

    pub fn as_smartcard(&self) -> &Smartcard {
        self.as_ref()
    }

    pub fn as_smartcard_mut(&mut self) -> &mut Smartcard {
        self.as_mut()
    }
}

impl AsRef<Smartcard> for Solo2 {
    fn as_ref(&self) -> &Smartcard {
        &self.card
    }
}

impl AsMut<Smartcard> for Solo2 {
    fn as_mut(&mut self) -> &mut Smartcard {
        &mut self.card
    }
}

impl TryFrom<Smartcard> for Solo2 {
    type Error = Error;
    fn try_from(card: Smartcard) -> Result<Solo2> {
        let mut card = card;
        let uuid = card.try_uuid()?;
        Ok(Solo2 { card, uuid })
    }
}

/// A SoloKeys Solo 2 device, which may be in regular ([Solo2]) or update ([Bootloader]) mode.
///
/// Not every [Smartcard] is a [Device]; currently if it reacts to the SoloKeys administrative
/// [App][crate::apps::admin::App] with a valid UUID, then we treat it as such.
// #[derive(Debug, Eq, PartialEq)]
pub enum Device {
    Bootloader(Bootloader),
    Solo2(Solo2),
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Device::*;
        match self {
            Bootloader(bootloader) => bootloader.fmt(f),
            Solo2(solo2) => solo2.fmt(f),
        }
    }
}

impl UuidSelectable for Device {
    fn try_uuid(&mut self) -> Result<Uuid> {
        Ok(self.uuid())
    }

    fn list() -> Vec<Self> {
        let bootloaders = Bootloader::list().into_iter().map(Device::from);
        let cards = Solo2::list().into_iter().map(Device::from);
        bootloaders.chain(cards).collect()
    }

    /// Fails is if zero or >1 devices have the given UUID.
    fn having(uuid: Uuid) -> Result<Self> {
        let mut candidates: Vec<Device> = Self::list().into_iter().filter(|card| card.uuid() == uuid).collect();
        match candidates.len() {
            0 => Err(anyhow!("No usable device has UUID {:X}", uuid.to_simple())),
            1 => Ok(candidates.remove(0)),
            n => Err(anyhow!("Multiple ({}) devices have UUID {:X}", n, uuid.to_simple())),
        }
    }

}

impl Device {
    fn uuid(&self) -> Uuid {
        match self {
            Device::Bootloader(bootloader) => Uuid::from_u128(bootloader.uuid),
            Device::Solo2(solo2) => solo2.uuid(),
        }
    }

    pub fn solo2(self) -> Result<Solo2> {
        match self {
            Device::Solo2(solo2) => Ok(solo2),
            _ => Err(anyhow!("This device is in bootloader mode.")),
        }
    }

    pub fn bootloader(self) -> Result<Bootloader> {
        match self {
            Device::Bootloader(bootloader) => Ok(bootloader),
            _ => Err(anyhow!("This device is not in bootloader mode.")),
        }
    }

    pub fn program(self, firmware: Firmware, skip_major_prompt: bool) -> Result<()> {
        use crate::{
            apps::App as _,
            Version,
        };

        let bootloader = match self {
            Device::Bootloader(bootloader) => bootloader,
            Device::Solo2(solo2) => {
                let uuid = solo2.uuid();
                // let uuid = lpc55::uuid::Builder::from_bytes(*uuid.as_bytes()).build();
                let mut admin = crate::apps::admin::App::with(solo2.into_inner());
                admin.select()?;
                let device_version: Version = admin.version()?;
                let new_version = firmware.version();

                info!("current device version: {}", device_version.to_calver());
                info!("new firmware version: {}", new_version.to_calver());

                if !skip_major_prompt {
                    if new_version.major > device_version.major {
                        use dialoguer::{theme, Confirm};
                        println!("Warning: This is is major update and it could risk breaking any current credentials on your key.");
                        println!("Check latest release notes here to double check: https://github.com/solokeys/solo2/releases");
                        println!("If you haven't used your key for anything yet, you can ignore this.\n");

                        if Confirm::with_theme(&theme::ColorfulTheme::default())
                            .with_prompt("Continue?")
                            .wait_for_newline(true)
                            .interact()?
                        {
                            println!("Continuing");
                        } else {
                            return Err(anyhow!("User aborted."));
                        }
                    }
                }

                // ignore errors based on dropped connection
                // TODO: should we raise others?
                admin.boot_to_bootrom().ok();

                println!("Waiting for key to enter bootloader mode...");

                // Wait for new bootloader to enumerate
                std::thread::sleep(std::time::Duration::from_millis(100));

                info!("attempt {}", 0);
                let mut bootloader = Bootloader::having(uuid);

                let mut attempts: i32 = 10;
                while bootloader.is_err() && attempts > 0 {
                    info!("attempt {}", 11 - attempts);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                    bootloader = Bootloader::having(uuid);
                    attempts -= 1;
                }

                bootloader?
            }
        };

        println!("Bootloader detected. The LED should be off.");
        println!("Writing new firmware...");
        firmware.write_to(&bootloader);

        println!("Done. Rebooting key.  The LED should turn back on.");
        bootloader.reboot();

        Ok(())
    }
}

impl From<Bootloader> for Device {
    fn from(bootloader: Bootloader) -> Device {
        Device::Bootloader(bootloader)
    }
}

impl From<Solo2> for Device {
    fn from(solo2: Solo2) -> Device {
        Device::Solo2(solo2)
    }
}
