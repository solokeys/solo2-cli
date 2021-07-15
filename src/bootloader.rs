use anyhow::anyhow;
use lpc55::bootloader::Bootloader;


/// Return a specific bootloader that is connected.
/// If no uuid is specified and there are multiple connected, the user will be prompted.
pub fn find_bootloader(uuid: Option<[u8; 16]>) -> crate::Result<Bootloader> {
	let mut bootloaders =
		Bootloader::list();

	if let Some(uuid) = uuid {
		let uuid_native = u128::from_be_bytes(uuid);
		for bootloader in bootloaders {
			if bootloader.uuid == uuid_native {
				return Ok(bootloader);
			}
		}
        return Err(anyhow!("Could not find any Solo 2 device with uuid {}.", hex::encode(uuid)));
	} else {
		use std::io::{stdin,stdout,Write};

		println!(
	"Multiple Trussed compatible devices connected.
	Enter 1-{} to select: ",
			bootloaders.len()
		);

		for i in 0 .. bootloaders.len() {
			println!("{} - UUID: {}", i, hex::encode(bootloaders[i].uuid.to_be_bytes()));
		}

		print!("Selection (0-9): ");
		stdout().flush().unwrap();

		let mut input = String::new();
		stdin().read_line(&mut input).expect("Did not enter a correct string");

		// remove whitespace
		input.retain(|c| !c.is_whitespace());

		let index: usize = input.parse().unwrap();

		if index > (bootloaders.len() - 1) {
			return Err(anyhow::anyhow!("Incorrect selection ({})", input));
		} else {
			return Ok(bootloaders.remove(index))
		}
	}

}
