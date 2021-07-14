use hex_literal::hex;

use crate::{Card, Result};
use pcsc::{Context, Protocols, Scope, ShareMode};

pub mod admin;
pub mod ndef;
pub mod oath;
pub mod piv;
pub mod provisioner;
pub mod tester;

pub const NFC_FORUM_RID: &'static [u8] = &hex!("D276000085");
pub const NIST_RID: &'static [u8] = &hex!("A000000308");
pub const SOLOKEYS_RID: &'static [u8] = &hex!("A000000847");
pub const YUBICO_RID: &'static [u8] = &hex!("A000000527");

pub const ADMIN_PIX: &'static [u8] = &hex!("00000001");
pub const NDEF_PIX: &'static [u8] = &hex!("0101");
pub const OATH_PIX: &'static [u8] = &hex!("2101");
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

    fn connect(uuid: Option<[u8; 16]>) -> Result<Card> {
        let context = Context::establish(Scope::User)?;
        let l = context.list_readers_len()?;
        let mut buffer = Vec::with_capacity(l);
        buffer.resize(l, 0);

        let readers = context.list_readers(&mut buffer)?.collect::<Vec<_>>();
        let mut cards_with_trussed = Vec::<Card>::new();

        for reader in readers {
            info!("connecting with reader: `{}`", &reader.to_string_lossy());
            let mut card = Card::from(context.connect(reader, ShareMode::Shared, Protocols::ANY)?);
            info!("...connected");

            match card.try_reading_uuid() {
                Ok(_uuid) => {
                    cards_with_trussed.push(card);
                    debug!("Reader is Trussed compatible.");
                },
                Err(_err) => {
                    // Not a Trussed supported device.
                    info!("Reader is not Trussed compatible: {:?}.", _err);
                }
            }
        }

        if cards_with_trussed.len() == 0 {
            return Err(anyhow::anyhow!("Could not find any Solo 2 devices connected."));
        }

        if cards_with_trussed.len() > 1 {
            if uuid.is_some() {
                // Just use this one.
                for card in cards_with_trussed {
                    if card.last_read_uuid() == uuid {
                        return Ok(card);
                    }
                }

                return Err(anyhow::anyhow!("Could not find any Solo 2 device with uuid {}.", hex::encode(uuid.unwrap())));

            } else {
                prompt_user_to_pick_trussed_device(&mut cards_with_trussed)
            }
        } else {
            // Only one card, use it.
            Ok(cards_with_trussed.remove(0))
        }

    }

    fn new(uuid: Option<[u8; 16]>) -> Result<Self>;

    fn call(&mut self, instruction: u8) -> Result<Vec<u8>> {
        self.card().call(0, instruction, 0x00, 0x00, None)
    }

    fn call_with(&mut self, instruction: u8, data: &[u8]) -> Result<Vec<u8>> {
        self.card().call(0, instruction, 0x00, 0x00, Some(&data))
    }

    fn print_aid() {
        println!("{}", hex::encode(Self::aid()).to_uppercase());
    }
}

fn prompt_user_to_pick_trussed_device(cards_with_trussed: &mut Vec<Card>) -> crate::Result<Card> {
    use std::io::{stdin,stdout,Write};

    println!(
"Multiple Trussed compatible devices connected.
Enter 1-{} to select: ",
        cards_with_trussed.len()
    );

    for i in 0 .. cards_with_trussed.len() {
        println!("{} - UUID: {}", i, hex::encode(cards_with_trussed[i].last_read_uuid().unwrap()));
    }

    print!("Selection (0-9): ");
    stdout().flush().unwrap();

    let mut input = String::new();
    stdin().read_line(&mut input).expect("Did not enter a correct string");

    // remove whitespace
    input.retain(|c| !c.is_whitespace());

    let index: usize = input.parse().unwrap();

    if index > (cards_with_trussed.len() - 1) {
        return Err(anyhow::anyhow!("Incorrect selection ({})", input));
    } else {
        Ok(cards_with_trussed.remove(index))
    }
}
