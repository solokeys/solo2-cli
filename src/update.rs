use std::{thread, time};

use anyhow::anyhow;
use lpc55::{bootloader::Bootloader, secure_binary::Version};

use crate::{
    apps::{admin, App},
    Card, Uuid,
    device::{prompt_user_to_select_device, Device},
    Result,
    smartcard,
};

pub mod assets;
use assets::LatestAssets;

#[derive(Clone, Eq, PartialEq)]
pub struct Firmware {
    sbfile: Vec<u8>,
}

impl Firmware {
    /// This is somewhat useless, we should instead verify the signatures on the SB2.1 file.
    pub fn verify_hexhash(&self, sha256_hex_hash: &str) -> Result<()> {
        use crypto::digest::Digest;
        use crypto::sha2::Sha256;

        let mut hasher = Sha256::new();
        hasher.input(&self.sbfile);

        (hasher.result_str() == sha256_hex_hash)
            .then(|| ())
            .ok_or_else(|| anyhow!("Sha2 hash on downloaded firmware did not verify!"))

    }

    pub fn read_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Ok(Self { sbfile: std::fs::read(path)? })
    }

    pub fn download_latest() -> Result<Self> {
        let specs = LatestAssets::fetch_spec()?;
        specs.fetch_firmware()
    }

    /// Belongs more logically as a Device method.
    pub fn program(&self, device: Device) -> Result<()> {
        let bootloader = match device {
            Device::Bootloader(bootloader) => bootloader,
            Device::Card(card) => {
                let uuid = card.uuid.unwrap();
                let uuid = lpc55::uuid::Builder::from_bytes(*uuid.as_bytes()).build();
                let mut admin = admin::App { card };
                admin.select()?;
                let device_version: Version = admin.version()?;
                let header_bytes = &self.sbfile.as_slice()[..96];
                let new_version = lpc55::secure_binary::Sb2Header::from_bytes(header_bytes)
                        .unwrap()
                        .product_version();

                info!("current device version: {}", device_version.to_calver());
                info!("new firmware version: {}", new_version.to_calver());

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

                admin.boot_to_bootrom().ok();

                println!("Waiting for key to enter bootloader mode...");

                // Wait for new bootloader to enumerate
                thread::sleep(time::Duration::from_millis(100));

                info!("attempt {}", 0);
                let mut bootloader = Bootloader::try_find(None, None, Some(uuid));

                let mut attempts: i32 = 10;
                while bootloader.is_err() && attempts > 0 {
                    info!("attempt {}", 11 - attempts);
                    thread::sleep(time::Duration::from_millis(100));
                    bootloader = Bootloader::try_find(None, None, Some(uuid));
                    attempts -= 1;
                }

                bootloader?
            }
        };

        println!("Bootloader detected. The LED should be off.");
        println!("Writing new firmware...");
        bootloader.receive_sb_file(&self.sbfile);

        println!("Done. Rebooting key.  The LED should turn back on.");
        bootloader.reboot();

        Ok(())
    }
}

// A rather tolerant update function, intended to be used by end users.
pub fn run_update_procedure(
    sbfilepath: Option<String>,
    uuid: Option<Uuid>,
    _skip_major_prompt: bool,
    update_all: bool,
) -> Result<()> {
    let solo_cards = Card::list(smartcard::Filter::SoloCards);

    let firmware: Firmware = sbfilepath
        .map(Firmware::read_from_file)
        .unwrap_or_else(|| {
            println!("Downloading latest release from https://github.com/solokeys/solo2/");
            Firmware::download_latest()
        })?;

    let bootloaders = Bootloader::list();
    let mut devices: Vec<Device> = Default::default();
    for card in solo_cards {
        devices.push(Device::Card(card))
    }
    for bootloader in bootloaders {
        devices.push(Device::Bootloader(bootloader))
    }

    if let Some(uuid) = uuid {
        for device in devices {
            match device.uuid() {
                Ok(device_uuid) => {
                    if device_uuid == uuid {
                        return firmware.program(device);
                    }
                }
                _ => continue,
            }
        }
        return Err(anyhow!("Cannot find solo2 device with UUID {:X}", uuid));
    } else if update_all {
        for device in devices {
            firmware.program(device)?;
        }
    } else {
        let device = prompt_user_to_select_device(devices)?;
        firmware.program(device)?;
    }
    Ok(())
}

