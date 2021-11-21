app_boilerplate!();

impl crate::App for App {
    const RID: &'static [u8] = super::SOLOKEYS_RID;
    const PIX: &'static [u8] = super::TESTER_PIX;
}
