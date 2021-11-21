//! Signed firmware releases for Solo 2 devices.
//!
use anyhow::anyhow;
/// Version of a firmware
pub use lpc55::secure_binary::Version;

use crate::Result;

pub mod github;

#[derive(Clone, Eq, PartialEq)]
pub struct Firmware {
    content: Vec<u8>,
    version: Version,
}

impl Firmware {
    pub fn version(&self) -> Version {
        self.version
    }

    // TODO: To implement this, upstream `lpc55` library needs to gain support
    // for decoding signed SB2.1 files.
    //
    // /// Returns Ok if the firmware has a valid signature.
    // pub fn verify(&self) -> Result<()> {
    // }

    pub fn write_to(&self, bootloader: &lpc55::Bootloader) {
        bootloader.receive_sb_file(&self.content);
    }

    /// This is not the best we can do; should instead verify the Firmware data is valid, and the signatures verify against [`R1`][crate::pki::Authority::R1].
    pub fn verify_hexhash(&self, sha256_hex_hash: &str) -> Result<()> {
        use crypto::digest::Digest;
        use crypto::sha2::Sha256;

        let mut hasher = Sha256::new();
        hasher.input(&self.content);

        (hasher.result_str() == sha256_hex_hash)
            .then(|| ())
            .ok_or_else(|| anyhow!("Sha2 hash on downloaded firmware did not verify!"))
    }

    pub fn new(content: Vec<u8>) -> Result<Self> {
        let header_bytes = &content.as_slice()[..96];
        let header = lpc55::secure_binary::Sb2Header::from_bytes(header_bytes)?;

        Ok(Self {
            content,
            version: header.product_version(),
        })
    }

    pub fn read_from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        Self::new(std::fs::read(path)?)
    }

    pub fn download_latest() -> Result<Self> {
        let specs = github::Release::fetch_spec()?;
        specs.fetch_firmware()
    }

}
