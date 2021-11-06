use core::fmt;

use anyhow::anyhow;
use flexiber::{Encodable, TaggedSlice};

use crate::{apps::App as _, Card, Error, Result};

pub struct App {
    pub card: Card,
}

impl super::App for App {
    const RID: &'static [u8] = super::YUBICO_RID;
    const PIX: &'static [u8] = super::OATH_PIX;

    fn new(uuid: Option<[u8; 16]>) -> Result<Self> {
        Ok(Self {
            card: Self::connect(uuid)?,
        })
    }

    fn card(&mut self) -> &mut Card {
        &mut self.card
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Hotp {
    pub initial_counter: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Totp {
    pub period: u32,
}

impl Default for Totp {
    fn default() -> Self {
        Self { period: 30 }
    }
}

#[derive(Clone, PartialEq)]
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

#[derive(Clone, Copy, Debug, PartialEq)]
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

#[derive(Clone, PartialEq)]
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
    /// output size is first shortened by applying the digest.
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
    fn from_base32(encoded: &str, digest: Digest) -> Result<Self> {
        let unshortened = data_encoding::BASE32.decode(encoded.as_bytes())?;
        let mut shortened = match digest {
            Digest::Sha1 => {
                use sha1::{Digest, Sha1};
                if unshortened.len() > Sha1::output_size() {
                    trace!(
                        "shortening {} as {} > {}",
                        hex::encode(&unshortened),
                        unshortened.len(),
                        Sha1::output_size()
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
                if unshortened.len() > Sha256::output_size() {
                    trace!(
                        "shortening {} as {} > {}",
                        hex::encode(&unshortened),
                        unshortened.len(),
                        Sha256::output_size()
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

#[derive(Clone, Debug, PartialEq)]
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

// #[derive(Clone, Debug, PartialEq)]
// pub struct CredentialId {
//     pub label: String,
//     gt
// }

impl Credential {
    pub fn id(&self) -> String {
        let mut id = String::new();
        if let Kind::Totp(totp) = self.kind {
            if totp != Default::default() {
                id += &format!("{}/", totp.period);
            }
        }
        if let Some(issuer) = &self.issuer {
            id += &format!("{}:", issuer);
        }
        id += &self.label;
        id
    }

    pub fn key(&self) -> Vec<u8> {
        let mut key = Vec::new();
        key.push((u8::from(&self.kind) << 4) + self.algorithm as u8);
        key.push(self.digits);
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

pub enum Command {
    Register(Credential),
    // Authenticate(CredentialId),
    Authenticate(Authenticate),
    Delete(String),
    List,
    Reset,
}

#[derive(Clone, Copy, Debug, PartialEq)]
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

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum Instruction {
    Put = 0x1,
    Delete = 0x2,
    Reset = 0x4,
    List = 0xA1,
    Calculate = 0xA2,
}

impl flexiber::Encodable for Tag {
    fn encoded_length(&self) -> flexiber::Result<flexiber::Length> {
        Ok(1u8.into())
    }
    fn encode(&self, encoder: &mut flexiber::Encoder<'_>) -> flexiber::Result<()> {
        encoder.encode(&[*self as u8])
    }
}

impl flexiber::Decodable<'_> for Tag {
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

impl App {
    /// Returns the credential ID.
    pub fn register(&mut self, credential: Credential) -> Result<String> {
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

        self.call_with(Instruction::Put as u8, &data).map(drop)?;

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
            self.card()
                .call(0, Instruction::Calculate as u8, 0x00, 0x01, Some(&data))?;
        debug!("response: {}", hex::encode(&response));

        assert_eq!(response[0], 0x76);
        assert_eq!(response[1], 5);
        let digits = response[2] as usize;
        let truncated_code = u32::from_be_bytes(response[3..].try_into().unwrap());
        let code = (truncated_code & 0x7FFFFFFF) % 10u32.pow(digits as _);
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

        self.call_with(Instruction::Delete as u8, &data).map(drop)
    }

    pub fn list(&mut self) -> Result<()> {
        // let mut data = Vec::new();

        let response = self.call(Instruction::List as u8)?;
        if response.is_empty() {
            debug!("no credentials");
            return Ok(());
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
            info!("{:?}", &credential_id);
            if decoder.is_finished() {
                return Ok(());
            }
        }
    }

    pub fn reset(&mut self) -> Result<()> {
        self.card()
            .call(0, Instruction::Reset as u8, 0xDE, 0xAD, None)
            .map(drop)
        // _, self._salt, self._challenge = _parse_select(self.protocol.select(AID.OATH))
    }
}

impl TryFrom<&clap::ArgMatches<'_>> for Command {
    type Error = Error;
    fn try_from(args: &clap::ArgMatches) -> Result<Self> {
        if let Some(args) = args.subcommand_matches("register") {
            trace!("{:?}", args);

            // the flags `--sha1` and `--sha256` must override the value of `--algorithm`,
            // which is always Some as it has a default.
            let digest = args
                .is_present("sha256")
                .then(|| "SHA256")
                .or_else(|| args.is_present("sha1").then(|| "SHA1"))
                .or_else(|| args.value_of("algorithm"))
                .unwrap()
                .try_into()?;

            let kind = args
                .is_present("hotp")
                .then(|| "HOTP")
                .or_else(|| args.is_present("totp").then(|| "TOTP"))
                .or_else(|| args.value_of("kind"))
                .unwrap()
                .to_uppercase();
            let kind = match kind.as_str() {
                "HOTP" => Kind::Hotp(Hotp {
                    initial_counter: args.value_of("counter").unwrap().parse()?,
                }),
                "TOTP" => Kind::Totp(Totp {
                    period: args.value_of("period").unwrap().parse()?,
                }),
                _ => return Err(anyhow!("clap is not doing its job")),
            };

            let credential = Credential {
                label: args.value_of("label").unwrap().to_string(),
                issuer: args.value_of("issuer").map(str::to_string),
                secret: Secret::from_base32(&args.value_of("secret").unwrap(), digest)?,
                kind,
                algorithm: digest,
                digits: args.value_of("digits").unwrap().parse().unwrap(),
            };
            return Ok(Command::Register(credential));
        }

        if let Some(args) = args.subcommand_matches("totp") {
            use std::time::SystemTime;

            let timestamp = args
                .value_of("timestamp")
                .map(|s| s.parse())
                .transpose()?
                .unwrap_or_else(|| {
                    let since_epoch = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap();
                    since_epoch.as_secs()
                });
            let authenticate = Authenticate {
                label: args.value_of("label").unwrap().to_string(),
                timestamp,
            };
            return Ok(Command::Authenticate(authenticate));
        }

        if let Some(args) = args.subcommand_matches("delete") {
            let label = args.value_of("label").unwrap().to_string();
            return Ok(Command::Delete(label));
        }

        if args.subcommand_matches("list").is_some() {
            return Ok(Command::List);
        }

        if args.subcommand_matches("reset").is_some() {
            return Ok(Command::Reset);
        }

        Err(anyhow!(
            "unexpected error, clap should enforce exhaustive match"
        ))
    }
}
