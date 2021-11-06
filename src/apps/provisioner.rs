use core::convert::TryInto;

use anyhow::anyhow;
use iso7816::Instruction;

use super::App as _;
use crate::{Card, Result};

pub struct App {
    pub card: Card,
}

impl super::App for App {
    const RID: &'static [u8] = super::SOLOKEYS_RID;
    const PIX: &'static [u8] = super::PROVISIONER_PIX;

    fn new(uuid: Option<[u8; 16]>) -> Result<Self> {
        Ok(Self {
            card: Self::connect(uuid)?,
        })
    }

    fn card(&mut self) -> &mut Card {
        &mut self.card
    }
}

impl App {
    // seems to be destructive currently
    // const BOOT_TO_BOOTROM_COMMAND: u8 = 0x51;
    const GENERATE_P256_ATTESTATION: u8 = 0xbc;
    const GENERATE_ED255_ATTESTATION: u8 = 0xbb;
    const GENERATE_X255_ATTESTATION: u8 = 0xb7;
    const BOOT_TO_BOOTROM: u8 = 0x51;
    const GET_UUID: u8 = 0x62;
    const REFORMAT_FS: u8 = 0xbd;
    const STORE_P256_ATTESTATION_CERT: u8 = 0xba;
    const STORE_ED255_ATTESTATION_CERT: u8 = 0xb9;
    const STORE_X255_ATTESTATION_CERT: u8 = 0xb6;
    const STORE_T1_INTERMEDIATE_PUBKEY: u8 = 0xb5;
    const WRITE_FILE: u8 = 0xbf;

    const PATH_ID: [u8; 2] = [0xe1, 0x01];
    const DATA_ID: [u8; 2] = [0xe1, 0x02];

    pub fn generate_trussed_ed255_attestation_key(&mut self) -> Result<[u8; 32]> {
        Ok(self
            .call(Self::GENERATE_ED255_ATTESTATION)?
            .as_slice()
            .try_into()?)
    }

    pub fn generate_trussed_p256_attestation_key(&mut self) -> Result<[u8; 64]> {
        Ok(self
            .call(Self::GENERATE_P256_ATTESTATION)?
            .as_slice()
            .try_into()?)
    }

    pub fn generate_trussed_x255_attestation_key(&mut self) -> Result<[u8; 32]> {
        Ok(self
            .call(Self::GENERATE_X255_ATTESTATION)?
            .as_slice()
            .try_into()?)
    }

    pub fn reformat_filesystem(&mut self) -> Result<()> {
        self.call(Self::REFORMAT_FS).map(drop)
    }

    pub fn store_trussed_ed255_attestation_certificate(&mut self, der: &[u8]) -> Result<()> {
        self.call_with(Self::STORE_ED255_ATTESTATION_CERT, der)
            .map(drop)
    }

    pub fn store_trussed_p256_attestation_certificate(&mut self, der: &[u8]) -> Result<()> {
        self.call_with(Self::STORE_P256_ATTESTATION_CERT, der)
            .map(drop)
    }

    pub fn store_trussed_x255_attestation_certificate(&mut self, der: &[u8]) -> Result<()> {
        self.call_with(Self::STORE_X255_ATTESTATION_CERT, der)
            .map(drop)
    }

    pub fn store_trussed_t1_intermediate_public_key(&mut self, public_key: [u8; 32]) -> Result<()> {
        self.call_with(Self::STORE_T1_INTERMEDIATE_PUBKEY, &public_key)
            .map(drop)
    }

    pub fn boot_to_bootrom(&mut self) -> Result<()> {
        self.call(Self::BOOT_TO_BOOTROM)?;
        Ok(())
    }

    pub fn uuid(&mut self) -> Result<u128> {
        let version_bytes = self.call(Self::GET_UUID)?;
        let bytes: &[u8] = &version_bytes;
        bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("expected 16 byte UUID, got {}", &hex::encode(bytes)))
            .map(u128::from_be_bytes)
    }

    pub fn write_file(&mut self, data: &[u8], path: &str) -> Result<()> {
        if data.len() > 8192 {
            return Err(anyhow!("data too long (8192 byte limit)"));
        }
        if path.as_bytes().len() > 128 {
            return Err(anyhow!("path {} too long (128 byte limit)"));
        }

        self.call_with(Instruction::Select.into(), &Self::PATH_ID)
            .map(drop)?;
        self.call_with(Instruction::WriteBinary.into(), path.as_bytes())
            .map(drop)?;

        self.call_with(Instruction::Select.into(), &Self::DATA_ID)
            .map(drop)?;
        self.call_with(Instruction::WriteBinary.into(), data)
            .map(drop)?;

        self.call(Self::WRITE_FILE).map(drop)
    }
}
