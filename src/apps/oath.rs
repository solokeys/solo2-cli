use core::fmt::{self, Write as _};

use anyhow::anyhow;
use flexiber::{Decodable, Encodable, TaggedSlice};

use crate::{Error, Result};

// pcsc_app!();
app!();

// impl<'t> crate::apps::PcscSelect<'t> for App<'t> {
impl<'t> crate::Select<'t> for App<'t> {
    const RID: &'static [u8] = super::Rid::YUBICO;
    const PIX: &'static [u8] = super::Pix::OATH;
    // fn select(transport: &'t mut dyn Transport) -> Result<Self> {
    //     return Err(anyhow::anyhow!("OATH app not supported on this transport"));
    // }
}

#[derive(Clone, Copy, Debug, Eq, Default, PartialEq)]
pub struct Hotp {
    pub initial_counter: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Totp {
    pub period: u32,
}

impl Default for Totp {
    fn default() -> Self {
        Self { period: 30 }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum Kind {
    Hotp(Hotp),
    Totp(Totp),
}

impl From<&Kind> for u8 {
    fn from(kind: &Kind) -> u8 {
        match kind {
            Kind::Hotp(_) => 0x1,
            Kind::Totp(_) => 0x2,
        }
    }
}

impl fmt::Debug for Kind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Hotp(hotp) => hotp.fmt(f),
            Self::Totp(totp) => totp.fmt(f),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Digest {
    Sha1 = 0x1,
    Sha256 = 0x2,
}

impl TryFrom<&str> for Digest {
    type Error = Error;
    fn try_from(name: &str) -> Result<Self> {
        Ok(match name.to_uppercase().as_ref() {
            "SHA1" => Self::Sha1,
            "SHA256" => Self::Sha256,
            name => return Err(anyhow!("Unknown or unimplemented hash algorithm {}", name)),
        })
    }
}

impl Default for Digest {
    fn default() -> Self {
        Self::Sha1
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct Secret(Vec<u8>);

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "'{}'", hex::encode(&self.0))
    }
}

impl Secret {
    const MINIMUM_SIZE: usize = 14;

    /// Decode the secret from a base 32 representation.
    ///
    /// Note: The secret is later used as an HMAC key.
    ///
    /// It is a property of HMAC that a key that is longer than the digest
    /// block size is first shortened by applying the digest. For SHA-1 and
    /// SHA-2, the block size is 64 bytes (512 bits).
    ///
    /// Therefore, applying the shortening in this implementation has no effect
    /// on the calculated OTP, but it does make communication with the OATH
    /// authenticator more efficient for oversized secrets.
    ///
    /// Note: The secret is always padded to at least 14 bytes with zero bytes,
    /// following `ykman`. This is a bit strange (?), as RFC 4226, section 4 says
    ///
    /// "The algorithm MUST use a strong shared secret.  The length of the shared
    /// secret MUST be least 128 bits.  This document RECOMMENDs a shared secret
    /// length of 160 bits."
    ///
    /// But 14B = 112b < 128b.
    pub fn from_base32(encoded: &str, digest: Digest) -> Result<Self> {
        let unshortened = data_encoding::BASE32.decode(encoded.as_bytes())?;
        let mut shortened = match digest {
            Digest::Sha1 => {
                use sha1::{Digest, Sha1};
                let block_size = 64;
                if unshortened.len() > block_size {
                    trace!(
                        "shortening {} as {} > {}",
                        hex::encode(&unshortened),
                        unshortened.len(),
                        block_size
                    );
                    let shortened = Sha1::digest(&unshortened).as_slice().to_vec();
                    trace!("...to {}", hex::encode(&shortened));
                    shortened
                } else {
                    unshortened
                }
            }
            Digest::Sha256 => {
                use sha2::{Digest, Sha256};
                let block_size = 64;
                if unshortened.len() > block_size {
                    trace!(
                        "shortening {} as {} > {}",
                        hex::encode(&unshortened),
                        unshortened.len(),
                        block_size
                    );
                    let shortened = Sha256::digest(&unshortened).as_slice().to_vec();
                    trace!("...to {}", hex::encode(&shortened));
                    shortened
                } else {
                    unshortened
                }
            }
        };

        shortened.resize(core::cmp::max(shortened.len(), Self::MINIMUM_SIZE), 0);

        Ok(Self(shortened))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Credential {
    // add device UUID/serial?
    // pub uuid: [u8; 16],
    pub label: String,
    pub issuer: Option<String>,
    pub secret: Secret,
    pub kind: Kind,
    pub algorithm: Digest,
    pub digits: u8,
}

impl Credential {
    pub fn default_totp(label: &str, secret32: &str) -> Result<Self> {
        let secret = Secret::from_base32(&secret32.to_uppercase(), Digest::default())?;

        Ok(Self {
            label: label.to_string(),
            issuer: None,
            secret,
            kind: Kind::Totp(Totp { period: 30 }),
            algorithm: Digest::default(),
            digits: 6,
        })
    }
}

// #[derive(Clone, Debug, PartialEq)]
// pub struct CredentialId {
//     pub label: String,
//     gt
// }

impl Credential {
    pub fn id(&self) -> String {
        let mut id = String::new();
        if let Kind::Totp(totp) = self.kind {
            if totp != Totp::default() {
                write!(id, "{}/", totp.period).ok();
            }
        }
        if let Some(issuer) = &self.issuer {
            write!(id, "{}:", issuer).ok();
        }
        id += &self.label;
        id
    }

    pub fn key(&self) -> Vec<u8> {
        let mut key = vec![
            (u8::from(&self.kind) << 4) + self.algorithm as u8,
            self.digits,
        ];
        key.extend_from_slice(&self.secret.0);

        key
    }
}

impl fmt::Display for Credential {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{}", self.id())
    }
}

pub struct Authenticate {
    pub label: String,
    pub timestamp: u64,
}

impl Authenticate {
    pub fn with_label(label: &str) -> Authenticate {
        use std::time::SystemTime;
        Self {
            label: label.to_string(),
            timestamp: {
                let since_epoch = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap();
                since_epoch.as_secs()
            },
        }
    }
}

pub enum Command {
    Register(Credential),
    // Authenticate(CredentialId),
    Authenticate(Authenticate),
    Delete(String),
    List,
    Reset,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Tag {
    CredentialId = 0x71,
    NameList = 0x72,
    Key = 0x73,
    Challenge = 0x74,
    InitialCounter = 0x7A,
}

impl TryFrom<u8> for Tag {
    type Error = Error;
    fn try_from(byte: u8) -> Result<Self> {
        use Tag::*;
        Ok(match byte {
            0x71 => CredentialId,
            0x72 => NameList,
            0x73 => Key,
            0x74 => Challenge,
            0x7A => InitialCounter,
            byte => return Err(anyhow!("Not a known tag: {}", byte)),
        })
    }
}

impl flexiber::TagLike for Tag {
    fn embedding(self) -> flexiber::Tag {
        // flexiber::SimpleTag::emb
        flexiber::Tag {
            class: flexiber::Class::Universal,
            constructed: false,
            number: self as u8 as u16,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum Instruction {
    Put = 0x1,
    Delete = 0x2,
    Reset = 0x4,
    List = 0xA1,
    Calculate = 0xA2,
}

impl Encodable for Tag {
    fn encoded_length(&self) -> flexiber::Result<flexiber::Length> {
        Ok(1u8.into())
    }
    fn encode(&self, encoder: &mut flexiber::Encoder<'_>) -> flexiber::Result<()> {
        encoder.encode(&[*self as u8])
    }
}

impl Decodable<'_> for Tag {
    fn decode(decoder: &mut flexiber::Decoder<'_>) -> flexiber::Result<Self> {
        use flexiber::TagLike;
        let simple_tag: flexiber::SimpleTag = decoder.decode()?;
        let byte = simple_tag.embedding().number as u8;
        let tag: Tag = byte
            .try_into()
            .map_err(|_| flexiber::Error::from(flexiber::ErrorKind::InvalidTag { byte }))?;
        Ok(tag)
    }
}

impl App<'_> {
    /// Returns the credential ID.
    pub fn register(&mut self, credential: &Credential) -> Result<String> {
        info!(" registering credential {:?}", &credential);
        // data = Tlv(TAG_NAME, cred_id) + Tlv(
        //     TAG_KEY,
        //     struct.pack("<BB", d.oath_type | d.hash_algorithm, d.digits) + secret,
        // )

        // if touch_required:
        //     data += struct.pack(b">BB", TAG_PROPERTY, PROP_REQUIRE_TOUCH)

        // if d.counter > 0:
        //     data += Tlv(TAG_IMF, struct.pack(">I", d.counter))

        // self.protocol.send_apdu(0, INS_PUT, 0, 0, data)

        let mut data = Vec::new();

        let credential_id = credential.id();
        debug!("credential ID: {}", credential_id);
        let credential_id_part = TaggedSlice::from(Tag::CredentialId, credential_id.as_bytes())
            .map_err(|e| e.kind())?
            .to_vec()
            .map_err(|e| e.kind())?;
        data.extend_from_slice(&credential_id_part);

        let key = credential.key();
        debug!("key: {}", hex::encode(&key));
        let key_part = TaggedSlice::from(Tag::Key, &key)
            .map_err(|e| e.kind())?
            .to_vec()
            .map_err(|e| e.kind())?;
        data.extend_from_slice(&key_part);

        if let Kind::Hotp(Hotp { initial_counter }) = credential.kind {
            let counter_part =
                TaggedSlice::from(Tag::InitialCounter, &initial_counter.to_be_bytes())
                    .map_err(|e| e.kind())?
                    .to_vec()
                    .map_err(|e| e.kind())?;
            data.extend_from_slice(&counter_part);
        }

        // TODO: touch....
        // if touch_required:
        //     data += struct.pack(b">BB", TAG_PROPERTY, PROP_REQUIRE_TOUCH)

        self.transport
            .call(Instruction::Put as u8, &data)
            .map(drop)?;

        Ok(credential_id)
    }

    /// Very limited implementation, more to come.
    ///
    /// Does *not* respect non-default TOTP periods.
    pub fn authenticate(&mut self, authenticate: Authenticate) -> Result<String> {
        let mut data = Vec::new();

        let credential_id = authenticate.label;
        debug!("credential ID: {}", credential_id);
        let credential_id_part = TaggedSlice::from(Tag::CredentialId, credential_id.as_bytes())
            .map_err(|e| e.kind())?
            .to_vec()
            .map_err(|e| e.kind())?;
        data.extend_from_slice(&credential_id_part);

        let challenge = authenticate.timestamp / (Totp::default().period as u64);
        let challenge_bytes = challenge.to_be_bytes();
        let challenge_part = TaggedSlice::from(Tag::Challenge, &challenge_bytes)
            .map_err(|e| e.kind())?
            .to_vec()
            .map_err(|e| e.kind())?;
        data.extend_from_slice(&challenge_part);

        let response =
            self.transport
                .call_iso(0, Instruction::Calculate as u8, 0x00, 0x01, &data)?;
        debug!("response: {}", hex::encode(&response));

        assert_eq!(response[0], 0x76);
        assert_eq!(response[1], 5);
        let digits = response[2] as usize;
        let truncated_code = u32::from_be_bytes(response[3..].try_into().unwrap());
        let code = (truncated_code & 0x7FFF_FFFF) % 10u32.pow(digits as _);
        Ok(format!("{:0digits$}", code, digits = digits))
    }

    pub fn delete(&mut self, label: String) -> Result<()> {
        let mut data = Vec::new();

        let credential_id = label;
        debug!("credential ID: {}", credential_id);
        let credential_id_part = TaggedSlice::from(Tag::CredentialId, credential_id.as_bytes())
            .map_err(|e| e.kind())?
            .to_vec()
            .map_err(|e| e.kind())?;
        data.extend_from_slice(&credential_id_part);

        self.transport
            .call(Instruction::Delete as u8, &data)
            .map(drop)
    }

    pub fn list(&mut self) -> Result<Vec<String>> {
        let mut labels = Vec::new();

        let response = self.transport.instruct(Instruction::List as u8)?;
        if response.is_empty() {
            debug!("no credentials");
            return Ok(labels);
        }
        debug!("{:?}", &hex::encode(&response));
        let mut decoder = flexiber::Decoder::new(response.as_slice());

        loop {
            let data = decoder
                .decode_tagged_slice(Tag::NameList)
                .map_err(|e| e.kind())?;
            // debug!("{:?}", &hex::encode(data));
            // let kind = data[0] ...
            let credential_id = std::str::from_utf8(&data[1..])?;
            trace!("{:?}", &credential_id);
            labels.push(credential_id.to_string());
            if decoder.is_finished() {
                return Ok(labels);
            }
        }
    }

    pub fn reset(&mut self) -> Result<()> {
        self.transport
            .call_iso(0, Instruction::Reset as u8, 0xDE, 0xAD, &[])
            .map(drop)
        // _, self._salt, self._challenge = _parse_select(self.protocol.select(AID.OATH))
    }
}
