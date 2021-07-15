use std::{thread, time};

use serde_json::{from_value, Value};
use anyhow::anyhow;
use lpc55::bootloader::Bootloader;

use crate::{Card, smartcard};
use crate::apps::App;
use crate::apps::admin;

pub enum SoloDevice {
    Card(Card),
    Bootloader(Bootloader),
}

impl SoloDevice {
    pub fn uuid(&self) -> u128 {
        match self {
            SoloDevice::Card(card) => card.uuid.unwrap(),
            SoloDevice::Bootloader(bootloader) => bootloader.uuid,
        }
    }
}

pub async fn download_latest_solokeys_firmware() -> crate::Result<Vec<u8>> {
    println!("Downloading latest release from https://github.com/solokeys/solo2/");

    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.github.com/repos/solokeys/solo2/releases/latest")
        .header("User-Agent", "solo2-cli")
        .send()
        .await?
        // .text()
        .json::<Value>()
        .await?
        ;

    let tagname: String = from_value(resp["tag_name"].clone()).unwrap();
    let assets: Vec<Value> = from_value(resp["assets"].clone()).unwrap();

    let mut sbfile: Option<Vec<u8>> = None;
    let mut sha256hash: Option<String> = None;

    println!("Downloading firmware v{}...",tagname);

    for asset in assets {
        let asset_name: String = from_value(asset["name"].clone()).unwrap();
        let asset_link: String = from_value(asset["browser_download_url"].clone()).unwrap();
        if asset_name == format!("solo2-firmware-{}.sb2", tagname) {
            info!("found solo2 firmare in release");
            sbfile = Some(client
                .get(asset_link.clone())
                .header("User-Agent", "solo2-cli")
                .send()
                .await?
                .bytes()
                .await?.to_vec());

        }
        if asset_name == format!("solo2-firmware-{}.sha2", tagname) {
            info!("found solo2 firmare hash in release");
            let hashfile = client
                .get(asset_link.clone())
                .header("User-Agent", "solo2-cli")
                .send()
                .await?
                .text()
                .await?;
            sha256hash = Some(hashfile.split(" ").collect::<Vec<&str>>()[0].into());
        }
    }

    if sbfile.is_none() || sha256hash.is_none() {
        return Err(anyhow!("Unable to find assets in latest SoloKeys release.  Please open ticket on solokeys.com/solo2 or contact hello@solokeys.com."));
    }

    use crypto::digest::Digest;
    use crypto::sha2::Sha256;

    let mut hasher = Sha256::new();
    hasher.input(sbfile.as_ref().unwrap());
    
    if hasher.result_str() != sha256hash.unwrap() {
        return Err(anyhow!("Sha2 hash on downloaded firmware did not verify!"));
    }
    println!("Verified hash.");

    Ok(sbfile.unwrap())
}

// A rather tolerant update function, intended to be used by end users.
pub async fn run_update_procedure (
    sbfile: Option<String>,
    uuid: Option<[u8; 16]>,
    _skip_major_prompt: bool,
    update_all: bool,
) -> crate::Result<()> {
    let trussed_cards = Card::list(smartcard::Filter::TrussedCards);

    let sbfile = if sbfile.is_none() {
        download_latest_solokeys_firmware().await?
    } else {
        std::fs::read(sbfile.unwrap())?
    };

    let bootloaders = Bootloader::list();
    let mut devices: Vec<SoloDevice> = Default::default();
    for card in trussed_cards {
        devices.push(SoloDevice::Card(card))
    }
    for bootloader in bootloaders {
        devices.push(SoloDevice::Bootloader(bootloader))
    }
    
    if let Some(uuid) = uuid {
        for device in devices {
            if device.uuid() == u128::from_be_bytes(uuid) {
                return program_device(device, sbfile)
            }
        }
        return Err(anyhow!("Cannot find solo2 device with UUID {}", hex::encode(uuid)))
    } else if update_all {
        for device in devices {
            program_device(device, sbfile.clone())?;
        }
    } else {
        let device = prompt_user_to_select_device(devices)?;
        program_device(device, sbfile)?;
    }
    // std::future::Future::Output(Ok(()));
    Ok(())
}

pub fn program_device(device: SoloDevice, sbfile: Vec<u8>) -> crate::Result<()> {
    let bootloader = match device {
        SoloDevice::Bootloader(bootloader) => {
            bootloader
        },
        SoloDevice::Card(card) => {
            let uuid = card.uuid.unwrap();
            let uuid = lpc55::uuid::Builder::from_bytes(uuid.to_be_bytes()).build();
            let mut admin = admin::App{ card };
            admin.select().ok();
            let device_version = u32::from_be_bytes(admin.version()?);
            let sb2_product_version = 
                lpc55::secure_binary::Sb2Header::from_bytes(&sbfile.as_slice()[0 .. 96])
                .unwrap()
                .product_version();

            // Device stores version as:
            //          major    minor   patch
            // bits:    10       16      6
            let device_version_major = device_version >> 22;
            info!("current device version major: {:?}", device_version_major);
            info!("new sb2 firmware version: {:?}", sb2_product_version);

            if device_version_major < sb2_product_version.major as u32 {
                use std::io::stdin;
                println!("Warning: This is is major update and it could risk breaking any current credentials on your key.");
                println!("Check latest release notes here to double check: https://github.com/solokeys/solo2/releases");
                println!("If you haven't used you key for anything yet, you can ignore this.");

                println!("");
                println!("Continue? y/Y: ");

                let mut input = String::new();
                stdin().read_line(&mut input).expect("Did not enter a correct string");

                // remove whitespace
                input.retain(|c| !c.is_whitespace());
                if ["y","yes"].contains(&input.to_ascii_lowercase().as_str()) {
                    println!("Continuing");
                } else {
                    return Err(anyhow!("User aborted."));
                }

            }
            admin.boot_to_bootrom().ok();

            // Wait for new bootloader to enumerate
            thread::sleep(time::Duration::from_millis(100));

            let mut bootloader = Bootloader::try_find(None, None, Some(uuid));
            
            let mut attempts: i32 = 4;
            while bootloader.is_err() && attempts > 0 {
                thread::sleep(time::Duration::from_millis(100));
                bootloader = Bootloader::try_find(None, None, Some(uuid));
                attempts -= 1;
            }

            bootloader?
        }
    };

    bootloader.receive_sb_file(sbfile);
    bootloader.reboot();

    Ok(())
}

/// Have user select device from currently connected devices.
pub fn prompt_user_to_select_device(mut devices: Vec<SoloDevice>) -> crate::Result<SoloDevice> {
    use std::io::{stdin,stdout,Write};

    println!(
"Multiple Solo 2 devices connected.
Enter 0-{} to select: ",
        devices.len()
    );

    for i in 0 .. devices.len() {
        match &devices[i] {
            SoloDevice::Bootloader(bootloader) => {
                println!(
                    "{} - Bootloader UUID: {}",
                    i,
                    hex::encode(bootloader.uuid.to_be_bytes())
                );

            },
            SoloDevice::Card(card) => {
                println!(
                    "{} - \"{}\" UUID: {}",
                    i, card.reader_name,
                    hex::encode(card.uuid.unwrap().to_be_bytes())
                );
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

