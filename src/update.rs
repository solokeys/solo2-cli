use anyhow::anyhow;
use lpc55::bootloader::Bootloader;


// A rather tolerant update function, intended to be used by end users.
pub fn run_update_procedure (_skip_major_prompt: bool) {
	let _bootloader =
		Bootloader::try_find(None, None, None)
			.or_else(|_| { Err(anyhow!("Could not attach to a bootloader")) });

}