//! Solo 2 devices, which may be in regular or bootloader mode.
use core::sync::atomic::{AtomicBool, Ordering};
use std::collections::{BTreeMap, BTreeSet};

use anyhow::anyhow;
use lpc55::bootloader::{Bootloader as Lpc55, UuidSelectable};

use crate::{apps::Admin, Firmware, Result, Select as _, Uuid, Version};
use core::fmt;

pub mod ctap;
pub mod pcsc;

/// A [SoloKeys][solokeys] [Solo 2][solo2] device, in regular mode.
///
/// From an inventory perspective, the core identifier is a UUID (16 bytes / 128 bits).
///
/// From an interface perspective, either the CTAP or PCSC transport must be available.
/// Therefore, it is an invariant that at least one is interface, and the device itself
/// implements [Transport][crate::Transport].
///
/// [solokeys]: https://solokeys.com
/// [solo2]: https://solo2.dev
pub struct Solo2 {
    ctap: Option<ctap::Device>,
    pcsc: Option<pcsc::Device>,
    uuid: Uuid,
    version: Version,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TransportPreference {
    Ctap,
    Pcsc,
}

static PREFER_CTAP: AtomicBool = AtomicBool::new(false);

impl Solo2 {
    pub fn transport_preference() -> TransportPreference {
        if PREFER_CTAP.load(Ordering::SeqCst) {
            TransportPreference::Ctap
        } else {
            TransportPreference::Pcsc
        }
    }

    pub fn prefer_ctap() {
        PREFER_CTAP.store(true, Ordering::SeqCst);
    }

    pub fn prefer_pcsc() {
        PREFER_CTAP.store(false, Ordering::SeqCst);
    }
}

impl fmt::Debug for Solo2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> core::result::Result<(), fmt::Error> {
        write!(
            f,
            "Solo 2 {:X} (CTAP: {:?}, PCSC: {:?}, Version: {} aka {})",
            &self.uuid.to_simple(),
            &self.ctap,
            &self.pcsc,
            &self.version.to_semver(),
            &self.version.to_calver(),
        )
    }
}

impl fmt::Display for Solo2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let transports = match (self.ctap.is_some(), self.pcsc.is_some()) {
            (true, true) => "CTAP+PCSC",
            (true, false) => "CTAP only",
            (false, true) => "PCSC only",
            _ => unreachable!(),
        };
        write!(
            f,
            "Solo 2 {:X} ({}, firmware {})",
            &self.uuid.to_simple(),
            transports,
            &self.version().to_calver()
        )
    }
}

impl UuidSelectable for Solo2 {
    fn try_uuid(&mut self) -> Result<Uuid> {
        Ok(self.uuid)
    }

    fn list() -> Vec<Self> {
        // iterator/lifetime woes avoiding the explicit for loop
        let mut ctaps = BTreeMap::new();
        for mut device in ctap::list().into_iter() {
            if let Ok(uuid) = device.try_uuid() {
                ctaps.insert(uuid, device);
            }
        }
        // iterator/lifetime woes avoiding the explicit for loop
        let mut pcscs = BTreeMap::new();
        for mut device in pcsc::list().into_iter() {
            if let Ok(uuid) = device.try_uuid() {
                pcscs.insert(uuid, device);
            }
        }

        let uuids = BTreeSet::from_iter(ctaps.keys().chain(pcscs.keys()).copied());
        let mut devices = Vec::new();
        for uuid in uuids.iter() {
            // a bit roundabout, but hey, "it works".
            let mut device = Self {
                ctap: ctaps.remove(uuid),
                pcsc: pcscs.remove(uuid),
                uuid: *uuid,
                version: Version {
                    major: 0,
                    minor: 0,
                    patch: 0,
                },
            };
            if let Ok(mut admin) = Admin::select(&mut device) {
                if let Ok(version) = admin.version() {
                    device.version = version;
                    devices.push(device);
                }
            }
        }
        devices
    }
}

impl Solo2 {
    /// UUID of device.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Firmware version on device.
    pub fn version(&self) -> Version {
        self.version
    }

    pub fn as_ctap(&self) -> Option<&ctap::Device> {
        self.ctap.as_ref()
    }

    pub fn as_ctap_mut(&mut self) -> Option<&mut ctap::Device> {
        self.ctap.as_mut()
    }

    pub fn as_pcsc(&self) -> Option<&pcsc::Device> {
        self.pcsc.as_ref()
    }

    pub fn as_pcsc_mut(&mut self) -> Option<&mut pcsc::Device> {
        self.pcsc.as_mut()
    }
}

impl TryFrom<ctap::Device> for Solo2 {
    type Error = crate::Error;
    fn try_from(device: ctap::Device) -> Result<Solo2> {
        let mut device = device;
        let uuid = device.try_uuid()?;
        let version = Admin::select(&mut device)?.version()?;

        Ok(Solo2 {
            ctap: Some(device),
            pcsc: None,
            uuid,
            version,
        })
    }
}

impl TryFrom<pcsc::Device> for Solo2 {
    type Error = crate::Error;
    fn try_from(device: pcsc::Device) -> Result<Solo2> {
        let mut device = device;
        let mut admin = Admin::select(&mut device)?;
        let uuid = admin.uuid()?;
        let version = admin.version()?;
        Ok(Solo2 {
            ctap: None,
            pcsc: Some(device),
            uuid,
            version,
        })
    }
}

/// A SoloKeys Solo 2 device, which may be in regular ([Solo2]) or update ([Lpc55]) mode.
///
/// Not every [pcsc::Device] is a [Device]; currently if it reacts to the SoloKeys administrative
/// [App][crate::apps::admin::App] with a valid UUID, then we treat it as such.
// #[derive(Debug, Eq, PartialEq)]
pub enum Device {
    Lpc55(Lpc55),
    Solo2(Solo2),
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use Device::*;
        match self {
            Lpc55(lpc55) => write!(f, "LPC 55 {:X}", Uuid::from_u128(lpc55.uuid).to_simple()),
            Solo2(solo2) => solo2.fmt(f),
        }
    }
}

impl UuidSelectable for Device {
    fn try_uuid(&mut self) -> Result<Uuid> {
        Ok(self.uuid())
    }

    fn list() -> Vec<Self> {
        let lpc55s = Lpc55::list().into_iter().map(Device::from);
        let solo2s = Solo2::list().into_iter().map(Device::from);
        lpc55s.chain(solo2s).collect()
    }

    /// Fails is if zero or >1 devices have the given UUID.
    fn having(uuid: Uuid) -> Result<Self> {
        let mut candidates: Vec<Device> = Self::list()
            .into_iter()
            .filter(|card| card.uuid() == uuid)
            .collect();
        match candidates.len() {
            0 => Err(anyhow!("No usable device has UUID {:X}", uuid.to_simple())),
            1 => Ok(candidates.remove(0)),
            n => Err(anyhow!(
                "Multiple ({}) devices have UUID {:X}",
                n,
                uuid.to_simple()
            )),
        }
    }
}

impl Device {
    fn uuid(&self) -> Uuid {
        match self {
            Device::Lpc55(lpc55) => Uuid::from_u128(lpc55.uuid),
            Device::Solo2(solo2) => solo2.uuid(),
        }
    }

    /// NB: will hang if in bootloader mode and Solo 2 firmware does not
    /// come up cleanly.
    pub fn into_solo2(self) -> Result<Solo2> {
        match self {
            Device::Solo2(solo2) => Ok(solo2),
            Device::Lpc55(lpc55) => {
                let uuid = Uuid::from_u128(lpc55.uuid);
                lpc55.reboot();

                let mut solo2 = Solo2::having(uuid);
                while solo2.is_err() {
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    solo2 = Solo2::having(uuid);
                }

                solo2
            }
        }
    }

    /// NB: Requires user tap if device is in Solo2 mode.
    pub fn into_lpc55(self) -> Result<Lpc55> {
        match self {
            Device::Lpc55(lpc55) => Ok(lpc55),
            Device::Solo2(mut solo2) => {
                let uuid = solo2.uuid;
                // AGAIN: This requires user tap!
                Admin::select(&mut solo2)?.boot_to_bootrom().ok();
                drop(solo2);

                std::thread::sleep(std::time::Duration::from_millis(1000));
                let mut lpc55 = Lpc55::having(uuid);
                while lpc55.is_err() {
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    lpc55 = Lpc55::having(uuid);
                }

                lpc55
            }
        }
    }

    pub fn program(self, firmware: Firmware, skip_major_prompt: bool) -> Result<()> {
        // If device is in Solo2 mode
        // - if firmware is major version bump, confirm with dialogue
        // - prompt user tap and get into bootloader
        // let device_version: Version = admin.version()?;
        // let new_version = firmware.version();

        let lpc55 = match self {
            Device::Lpc55(lpc55) => lpc55,
            Device::Solo2(solo2) => {
                // If device is in Solo2 mode
                // - if firmware is major version bump, confirm with dialogue
                // - prompt user tap and get into Lpc55 bootloader

                info!("device fw version: {}", solo2.version.to_calver());
                info!("new fw version: {}", firmware.version().to_calver());

                let fw_major = firmware.version().major;
                let major_version_bump = fw_major > solo2.version.major;
                if !skip_major_prompt && major_version_bump {
                    use dialoguer::{theme, Confirm};
                    println!("Warning: This is is major update and it could risk breaking any current credentials on your key.");
                    println!("Check latest release notes here to double check: https://github.com/solokeys/solo2/releases");
                    println!(
                        "If you haven't used your key for anything yet, you can ignore this.\n"
                    );

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

                println!("Tap button on key to confirm, or replug to abort...");
                Self::Solo2(solo2).into_lpc55()?
            }
        };

        println!("LPC55 Bootloader detected. The LED should be off.");
        println!("Writing new firmware...");
        firmware.write_to(&lpc55);

        println!("Done. Rebooting key.  The LED should turn back on.");
        Self::Lpc55(lpc55).into_solo2().map(drop)
    }
}

impl From<Lpc55> for Device {
    fn from(lpc55: Lpc55) -> Device {
        Device::Lpc55(lpc55)
    }
}

impl From<Solo2> for Device {
    fn from(solo2: Solo2) -> Device {
        Device::Solo2(solo2)
    }
}
