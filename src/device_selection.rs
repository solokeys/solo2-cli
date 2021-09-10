//! For selecting devices when there are potentially multiple connected.
//! Also for allowing devices to be selected whether if they are in bootloader mode or not.
use anyhow::anyhow;
use lpc55::bootloader::Bootloader;

use crate::{Card, Result, Uuid};

pub enum Device {
    Bootloader(Bootloader),
    Card(Card),
}

impl Device {
    /// If this is a Solo device, this will successfully report the UUID.
    /// Not guaranteed to work with other devices.
    pub fn uuid(&self) -> Result<Uuid> {
        match self {
            Device::Bootloader(bootloader) => Ok(bootloader.uuid.into()),
            Device::Card(card) => card.uuid.ok_or(anyhow!("Device does not have a UUID")),
        }
    }

    pub fn card(self) -> Result<Card> {
        match self {
            Device::Card(card) => Ok(card),
            _ => Err(anyhow!("This device is in bootloader mode.")),
        }
    }

    pub fn bootloader(self) -> Result<Bootloader> {
        match self {
            Device::Bootloader(bootloader) => Ok(bootloader),
            _ => Err(anyhow!("This device is not in bootloader mode.")),
        }
    }
}

impl From<Card> for Device {
    fn from(card: Card) -> Device {
        Device::Card(card)
    }
}

impl From<Bootloader> for Device {
    fn from(bootloader: Bootloader) -> Device {
        Device::Bootloader(bootloader)
    }
}

/// Return a specific bootloader that is connected.
/// If no uuid is specified and there are multiple connected, the user will be prompted.
pub fn find_bootloader(uuid: Option<Uuid>) -> Result<Bootloader> {
    let bootloaders = Bootloader::list();

    if let Some(uuid) = uuid {
        for bootloader in bootloaders {
            if bootloader.uuid == uuid.u128() {
                return Ok(bootloader);
            }
        }
        return Err(anyhow!("Could not find any Solo 2 device with uuid {}.", uuid.hex()));
    } else {
        let mut devices: Vec<Device> = Default::default();
        for bootloader in bootloaders {
            devices.push(bootloader.into())
        }

        let selected = prompt_user_to_select_device(devices)?;
        selected.bootloader()
    }
}

/// Have user select device from list of devices.
pub fn prompt_user_to_select_device(mut devices: Vec<Device>) -> Result<Device> {
    if devices.is_empty() {
        return Err(anyhow!("No Solo 2 devices connected"));
    }

    let items: Vec<String> = devices.iter().map(|device| {
        match device {
            Device::Bootloader(bootloader) => {
                format!(
                    "Bootloader UUID: {}",
                    hex::encode(bootloader.uuid.to_be_bytes())
                )

            },
            Device::Card(card) => {
                if let Some(uuid) = card.uuid {
                    // format!("\"{}\" UUID: {}", card.reader_name, hex::encode(uuid.to_be_bytes()))
                    format!("Solo 2 {}", uuid.hex())
                } else {
                    format!(" \"{}\"", card.reader_name)
                }
            }
        }
    }).collect();

    use dialoguer::{Select, theme};
    // let selection = Select::with_theme(&theme::SimpleTheme)
    let selection = Select::with_theme(&theme::ColorfulTheme::default())
        .with_prompt("Multiple Solo 2 devices connected, select one or hit Escape key")
        .items(&items)
        .default(0)
        .interact_opt()?
        .ok_or(anyhow!("No device selected"))?;

    Ok(devices.remove(selection))

}
