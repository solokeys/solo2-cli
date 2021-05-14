use hex_literal::hex;

use crate::{Card, Result};
use pcsc::{Context, Protocols, Scope, ShareMode};

pub mod management;
pub mod ndef;
pub mod piv;
pub mod provisioner;
pub mod tester;

pub const NFC_FORUM_RID: &'static [u8] = &hex!("D276000085");
pub const NIST_RID: &'static [u8] = &hex!("A000000308");
pub const SOLOKEYS_RID: &'static [u8] = &hex!("A000000847");

pub const MANAGEMENT_PIX: &'static [u8] = &hex!("00000001");
pub const NDEF_PIX: &'static [u8] = &hex!("010100");
// the full PIX ends with 0100 for version 01.00,
// truncated is enough to select
// pub const PIV_PIX: &'static [u8] = &hex!("000010000100");
pub const PIV_PIX: &'static [u8] = &hex!("00001000");
pub const PROVISIONER_PIX: &'static [u8] = &hex!("01000001");
pub const TESTER_PIX: &'static [u8] = &hex!("01000000");

pub trait App: Sized {
    const RID: &'static [u8];
    const PIX: &'static [u8];

    fn aid() -> Vec<u8> {
        let mut aid: Vec<u8> = Default::default();
        aid.extend_from_slice(Self::RID);
        aid.extend_from_slice(Self::PIX);
        aid
    }

    fn select(&mut self) -> Result<Vec<u8>> {
        // use iso7816::command::class::Class;
        info!("selecting app: {}", hex::encode(Self::aid()).to_uppercase());

        self.card().call(
            // Class::
            0,
            iso7816::Instruction::Select.into(),
            0x04,
            0x00,
            Some(&Self::aid()),
        )
    }

    fn card(&mut self) -> &mut Card;

    fn connect() -> Result<Card> {
        let context = Context::establish(Scope::User)?;
        let l = context.list_readers_len()?;
        let mut buffer = Vec::with_capacity(l);
        buffer.resize(l, 0);

        let readers = context.list_readers(&mut buffer)?.collect::<Vec<_>>();
        // TODO: select (by UUID)
        if readers.len() != 1 {
            return Err(anyhow::anyhow!("PCSC reader not unique"));
        }
        let reader = readers[0];

        info!("connecting with reader: `{}`", &reader.to_string_lossy());
        let card = Card::from(context.connect(reader, ShareMode::Shared, Protocols::ANY)?);
        info!("...connected");

        Ok(card)
    }

    fn new() -> Result<Self>;

    fn call(&mut self, instruction: u8) -> Result<Vec<u8>> {
        self.card().call(0, instruction, 0x00, 0x00, None)
    }

    fn call_with(&mut self, instruction: u8, data: &[u8]) -> Result<Vec<u8>> {
        self.card().call(0, instruction, 0x00, 0x00, Some(&data))
    }
}
