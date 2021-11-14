//! For selecting devices when there are potentially multiple connected.
//! Also for allowing devices to be selected whether if they are in bootloader mode or not.
use anyhow::anyhow;
use lpc55::bootloader::Bootloader;

use crate::{Card, Error, Result, Smartcard, Uuid};
use core::fmt;

pub struct Solo2 {
    card: Smartcard,
    uuid: Uuid,
}

impl fmt::Debug for Solo2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::result::Result<(), fmt::Error> {
        write!(f, "Solo 2 {:X} ({})", &self.uuid, &self.card.name)
    }
}


impl Solo2 {
    pub fn list() -> Vec<Self> {
        let smartcards = Smartcard::list();
        smartcards.into_iter()
            .filter_map(|card| Self::try_from(card).ok())
            .collect()
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
        use crate::apps;

        let mut aid: Vec<u8> = Default::default();
        aid.extend_from_slice(apps::SOLOKEYS_RID);
        aid.extend_from_slice(apps::ADMIN_PIX);

        let mut card = card;
        card.call(
            0, iso7816::Instruction::Select.into(),
            0x04, 0x00,
            Some(&aid),
        )?;

        let uuid_bytes: [u8; 16] = card.call(
            0, apps::admin::App::UUID_COMMAND,
            0x00, 0x00,
            None,
        )?
            .try_into()
            .map_err(|_| anyhow!("Did not read 16 byte uuid from mgmt app."))?;

        let uuid = Uuid::from_bytes(uuid_bytes);

        Ok(Solo2 { card, uuid })
    }
}

// #[derive(Debug, Eq, PartialEq)]
pub enum Device {
    Bootloader(Bootloader),
    Card(Card),
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Device::Bootloader(bootloader) => f.write_fmt(format_args!(
                "Bootloader UUID: {:X}",
                Uuid::from_u128(bootloader.uuid),
            )),
            Device::Card(card) => card.fmt(f),
        }
    }
}

impl Device {
    pub fn list() -> Vec<Self> {
        let bootloaders = Bootloader::list().into_iter().map(Device::from);
        let cards = Card::list(crate::smartcard::Filter::SoloCards)
            .into_iter()
            .map(Device::from);

        bootloaders.chain(cards).collect()
    }

    /// If this is a Solo device, this will successfully report the UUID.
    /// Not guaranteed to work with other devices.
    pub fn uuid(&self) -> Result<Uuid> {
        match self {
            Device::Bootloader(bootloader) => Ok(Uuid::from_u128(bootloader.uuid)),
            Device::Card(card) => card
                .uuid
                .ok_or_else(|| anyhow!("Device does not have a UUID")),
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
            if bootloader.uuid == uuid.as_u128() {
                return Ok(bootloader);
            }
        }
        return Err(anyhow!(
            "Could not find any Solo 2 device with uuid {:X}.",
            uuid,
        ));
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

    let items: Vec<String> = devices
        .iter()
        .map(|device| {
            match device {
                Device::Bootloader(bootloader) => {
                    format!(
                        "Bootloader UUID: {}",
                        hex::encode(bootloader.uuid.to_be_bytes())
                    )
                }
                Device::Card(card) => {
                    if let Some(uuid) = card.uuid {
                        // format!("\"{}\" UUID: {}", card.reader_name, hex::encode(uuid.to_be_bytes()))
                        format!("Solo 2 {:X}", uuid)
                    } else {
                        format!(" \"{}\"", card.reader_name)
                    }
                }
            }
        })
        .collect();

    use dialoguer::{theme, Select};
    // let selection = Select::with_theme(&theme::SimpleTheme)
    let selection = Select::with_theme(&theme::ColorfulTheme::default())
        .with_prompt("Multiple Solo 2 devices connected, select one or hit Escape key")
        .items(&items)
        .default(0)
        .interact_opt()?
        .ok_or_else(|| anyhow!("No device selected"))?;

    Ok(devices.remove(selection))
}
