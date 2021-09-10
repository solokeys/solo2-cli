use crate::{Card, Result, Uuid};

pub struct App {
    pub card: Card,
}

impl super::App for App {
    const RID: &'static [u8] = super::NIST_RID;
    const PIX: &'static [u8] = super::PIV_PIX;

    fn new(uuid: Option<Uuid>) -> Result<Self> {
        Ok(Self {
            card: Self::connect(uuid)?,
        })
    }

    fn card(&mut self) -> &mut Card {
        &mut self.card
    }
}
