#[macro_use]
extern crate log;

mod cli;

use core::convert::TryFrom;
use anyhow::anyhow;
use lpc55::bootloader::Bootloader;
use solo2;

fn main() {
    pretty_env_logger::init_custom_env("SOLO2_LOG");
    info!("solo2 CLI startup");

    let args = cli::cli().get_matches();

    if let Err(err) = try_main(args) {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

fn try_main(args: clap::ArgMatches<'_>) -> anyhow::Result<()> {
    if let Some(args) = args.subcommand_matches("app") {
        use solo2::apps::App;

        if let Some(args) = args.subcommand_matches("management") {
            info!("interacting with management app");

            let mut app = solo2::apps::management::App::new()?;
            let answer_to_select = app.select()?;
            info!("answer to select: {}", &hex::encode(answer_to_select));

            if args.subcommand_matches("boot-to-bootrom").is_some() {
                app.boot_to_bootrom()?;
            }
            if args.subcommand_matches("reboot").is_some() {
                info!("attempting reboot");
                app.reboot()?;
            }
            if args.subcommand_matches("uuid").is_some() {
                let uuid = app.uuid()?;
                println!("{}", hex::encode(uuid.to_le_bytes()));
            }
            if args.subcommand_matches("version").is_some() {
                let version = app.version()?;
                println!("{:?}", version);
            }

        }
    }
    if let Some(args) = args.subcommand_matches("bootloader") {
        // let bootloader = || Bootloader::try_find(Some(0x1209), Some(0xbeee), None)
        let bootloader = || Bootloader::try_find(None, None, None)
            .ok_or(anyhow!("Could not attach to a bootloader"));
        if args.subcommand_matches("reboot").is_some() {
            let bootloader = bootloader()?;
            bootloader.reboot();
        }
    }


    if let Some(command) = args.subcommand_matches("provision") {
        let config_filename = command.value_of("CONFIG").unwrap();
        let _config = lpc55::bootloader::provision::Config::try_from(config_filename)?;

        // let bootloader = bootloader()?;
        // for cmd in config.provisions {
        //     println!("cmd: {:?}", cmd);
        //     bootloader.run_command(cmd)?;
        // }

        return Ok(());
    }

    Ok(())
}
