use crate::Smartcard;

app_boilerplate!();

impl super::App for App {
    const RID: &'static [u8] = super::SOLOKEYS_RID;
    const PIX: &'static [u8] = super::TESTER_PIX;
}
