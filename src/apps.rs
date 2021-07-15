use hex_literal::hex;

use crate::{Card, Result};

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

        let mut cards = Card::list(Default::default());

        if cards.len() == 0 {
            return Err(anyhow::anyhow!("Could not find any Solo 2 devices connected."));
        }

        if cards.len() > 1 {
            if let Some(uuid) = uuid {
                // Just use this one.
                for card in cards {
                    if let Some(card_uuid) = card.uuid {
                        if card_uuid == u128::from_be_bytes(uuid) {
                            return Ok(card);
                        }
                    }
                }

                return Err(anyhow::anyhow!("Could not find any Solo 2 device with uuid {}.", hex::encode(uuid)));

            } else {
                prompt_user_to_pick_card(&mut cards)
            }
        } else {
            // Only one card, use it.
            Ok(cards.remove(0))
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

fn prompt_user_to_pick_card(cards: &mut Vec<Card>) -> crate::Result<Card> {
    use std::io::{stdin,stdout,Write};

    println!(
"Multiple smartcards connected.
Enter 0-{} to select: ",
        cards.len()
    );

    for i in 0 .. cards.len() {
        if let Some(uuid) = cards[i].uuid {
            println!("{} - \"{}\" UUID: {}", i, cards[i].reader_name, hex::encode(uuid.to_be_bytes()));
        } else {
            println!("{} - \"{}\"", i, cards[i].reader_name);
        }
    }

    print!("Selection (0-9): ");
    stdout().flush().unwrap();

    let mut input = String::new();
    stdin().read_line(&mut input).expect("Did not enter a correct string");

    // remove whitespace
    input.retain(|c| !c.is_whitespace());

    let index: usize = input.parse().unwrap();

    if index > (cards.len() - 1) {
        return Err(anyhow::anyhow!("Incorrect selection ({})", input));
    } else {
        Ok(cards.remove(index))
    }
}
