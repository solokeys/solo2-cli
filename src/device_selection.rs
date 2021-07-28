//! For selecting devices when there are potentially multiple connected.
//! Also for allowing devices to be selected whether if they are in bootloader mode or not.
use anyhow::anyhow;
use lpc55::bootloader::Bootloader;

use crate::{Card};

pub enum Device {
    Card(Card),
    Bootloader(Bootloader),
}

impl Device {
    /// If this is a Solo device, this will successfully report the UUID.
    /// Not guaranteed to work with other devices.
    pub fn uuid(&self) -> crate::Result<u128> {
        match self {
            Device::Card(card) => card.uuid.ok_or(anyhow!("Device does not have a UUID")),
            Device::Bootloader(bootloader) => Ok(bootloader.uuid),
        }
    }

    pub fn card(self) -> crate::Result<Card> {
        match self {
            Device::Card(card) => Ok(card),
            _ => Err(anyhow!("This device is in bootloader mode.")),
        }
    }

    pub fn bootloader(self) -> crate::Result<Bootloader> {
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
pub fn find_bootloader(uuid: Option<[u8; 16]>) -> crate::Result<Bootloader> {
    let bootloaders =
        Bootloader::list();

    if let Some(uuid) = uuid {
        let uuid_native = u128::from_be_bytes(uuid);
        for bootloader in bootloaders {
            if bootloader.uuid == uuid_native {
                return Ok(bootloader);
            }
        }
        return Err(anyhow!("Could not find any Solo 2 device with uuid {}.", hex::encode(uuid)));
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
pub fn prompt_user_to_select_device(mut devices: Vec<Device>) -> crate::Result<Device> {
    use std::io::{stdin,stdout,Write};

    println!(
"Multiple devices connected.
Enter 0-{} to select: ",
        devices.len()
    );

    for i in 0 .. devices.len() {
        match &devices[i] {
            Device::Bootloader(bootloader) => {
                println!(
                    "{} - Bootloader UUID: {}",
                    i,
                    hex::encode(bootloader.uuid.to_be_bytes())
                );

            },
            Device::Card(card) => {
                if let Some(uuid) = card.uuid {
                    println!("{} - \"{}\" UUID: {}", i, card.reader_name, hex::encode(uuid.to_be_bytes()));
                } else {
                    println!("{} - \"{}\"", i, card.reader_name);
                }
            }
        };
    }

    print!("Selection (0-9): ");
    stdout().flush().unwrap();

    let mut input = String::new();
    stdin().read_line(&mut input).expect("Did not enter a correct string");

    // remove whitespace
    input.retain(|c| !c.is_whitespace());

    let index: usize = input.parse().unwrap();

    if index > (devices.len() - 1) {
        return Err(anyhow::anyhow!("Incorrect selection ({})", input));
    } else {
        Ok(devices.remove(index))
    }

}

