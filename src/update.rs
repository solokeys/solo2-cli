use std::{thread, time};

use serde_json::{from_value, Value};
use anyhow::anyhow;
use lpc55::bootloader::Bootloader;

use crate::{Card, smartcard};
use crate::apps::App;
use crate::apps::admin;
use crate::device_selection::{Device, prompt_user_to_select_device};


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
    let solo_cards = Card::list(smartcard::Filter::SoloCards);

    let sbfile = if sbfile.is_none() {
        download_latest_solokeys_firmware().await?
    } else {
        std::fs::read(sbfile.unwrap())?
    };

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
            if device.uuid().unwrap() == u128::from_be_bytes(uuid) {
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

pub fn program_device(device: Device, sbfile: Vec<u8>) -> crate::Result<()> {
    let bootloader = match device {
        Device::Bootloader(bootloader) => {
            bootloader
        },
        Device::Card(card) => {
            let uuid = card.uuid.unwrap();
            let uuid = lpc55::uuid::Builder::from_bytes(uuid.to_be_bytes()).build();
            let mut admin = admin::App{ card };
            admin.select().ok();
            let device_version: u32 = admin.version()?.into();
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
                println!("If you haven't used your key for anything yet, you can ignore this.");

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

            println!("Waiting for key to enter bootloader mode...");

            // Wait for new bootloader to enumerate
            thread::sleep(time::Duration::from_millis(100));

            info!("attempt {}", 0);
            let mut bootloader = Bootloader::try_find(None, None, Some(uuid));

            let mut attempts: i32 = 10;
            while bootloader.is_err() && attempts > 0 {
                info!("attempt {}", 11-attempts);
                thread::sleep(time::Duration::from_millis(100));
                bootloader = Bootloader::try_find(None, None, Some(uuid));
                attempts -= 1;
            }

            bootloader?
        }
    };

    println!("Bootloader detected. The LED should be off.");
    println!("Writing new firmware...");
    bootloader.receive_sb_file(sbfile);

    println!("Done. Rebooting key.  The LED should turn back on.");
    bootloader.reboot();

    Ok(())
}

