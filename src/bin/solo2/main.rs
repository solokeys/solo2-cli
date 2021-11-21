#[macro_use]
extern crate log;

mod cli;

use anyhow::anyhow;
use lpc55::bootloader::{Bootloader, UuidSelectable};
use solo2::{Device, Uuid};

fn main() {
    pretty_env_logger::init_custom_env("SOLO2_LOG");
    restore_cursor_on_ctrl_c();

    let args = cli::cli().get_matches();

    if let Err(err) = try_main(args) {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

/// description: plural of thing to be selected, e.g. "Solo 2 devices"
pub fn interactively_select<T: core::fmt::Display>(
    candidates: Vec<T>,
    description: &str,
) -> anyhow::Result<T> {
    let mut candidates = match candidates.len() {
        0 => return Err(anyhow!("Empty list of {}", description)),
        1 => {
            let mut candidates = candidates;
            return Ok(candidates.remove(0));
        }
        _ => candidates,
    };

    let items: Vec<String> = candidates
        .iter()
        .map(|candidate| format!("{}", &candidate))
        .collect();

    use dialoguer::{theme, Select};
    // let selection = Select::with_theme(&theme::SimpleTheme)
    let selection = Select::with_theme(&theme::ColorfulTheme::default())
        .with_prompt(format!(
            "Multiple {} available, select one or hit Escape key",
            description
        ))
        .items(&items)
        .default(0)
        .interact_opt()?
        .ok_or_else(|| anyhow!("No candidate selected"))?;

    Ok(candidates.remove(selection))
}

pub fn unwrap_or_interactively_select<T: core::fmt::Display + UuidSelectable>(
    uuid: Option<Uuid>,
    description: &str,
) -> anyhow::Result<T> {
    let thing = match uuid {
        Some(uuid) => T::having(uuid)?,
        None => interactively_select(T::list(), description)?,
    };
    Ok(thing)
}

fn try_main(args: clap::ArgMatches<'_>) -> anyhow::Result<()> {
    let uuid = args
        .value_of("uuid")
        // if uuid is Some, parse and fail on invalidity (no silent failure)
        .map(|uuid| uuid.parse())
        .transpose()?;

    if let Some(args) = args.subcommand_matches("app") {
        use solo2::apps::App;

        if let Some(args) = args.subcommand_matches("admin") {
            info!("interacting with admin app");
            use solo2::apps::admin::App as AdminApp;

            if args.subcommand_matches("aid").is_some() {
                println!("{}", hex::encode(AdminApp::aid()).to_uppercase());
                return Ok(());
            }

            let card = unwrap_or_interactively_select(uuid, "smartcards")?;
            let mut app = AdminApp::with(card);
            let answer_to_select = app.select()?;
            info!("answer to select: {}", &hex::encode(answer_to_select));

            if args.subcommand_matches("boot-to-bootrom").is_some() {
                let uuid = app.uuid()?;
                // prompt first - on Windows, app call doesn't return immediately
                println!("Tap button on key to reboot, or replug to abort...");
                // ignore errors based on dropped connection
                // TODO: should we raise others?
                app.boot_to_bootrom().ok();

                while Bootloader::having(uuid).is_err() {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                }
                println!("...rebooted");
            }
            if args.subcommand_matches("reboot").is_some() {
                info!("attempting reboot");
                app.reboot()?;
            }
            if args.subcommand_matches("uuid").is_some() {
                let uuid = app.uuid()?;
                println!("{:X}", uuid.to_simple());
            }
            if args.subcommand_matches("version").is_some() {
                let version = app.version()?;
                println!("{}", version.to_calver());
            }
        }

        if let Some(args) = args.subcommand_matches("ndef") {
            info!("interacting with NDEF app");
            use solo2::apps::ndef::App as NdefApp;
            if args.subcommand_matches("aid").is_some() {
                println!("{}", hex::encode(NdefApp::aid()).to_uppercase());
                return Ok(());
            }

            let card = unwrap_or_interactively_select(uuid, "smartcards")?;
            let mut app = NdefApp::with(card);
            app.select()?;

            if args.subcommand_matches("capabilities").is_some() {
                let capabilities = app.capabilities()?;
                println!("{}", hex::encode(capabilities));
            }
            if args.subcommand_matches("data").is_some() {
                let data = app.data()?;
                println!("{}", hex::encode(data));
            }
        }

        if let Some(args) = args.subcommand_matches("oath") {
            info!("interacting with OATH app");
            use solo2::apps::oath::{App, Command};
            if args.subcommand_matches("aid").is_some() {
                App::print_aid();
                return Ok(());
            }

            let card = unwrap_or_interactively_select(uuid, "smartcards")?;
            let mut app = App::with(card);
            app.select()?;

            let command: Command = Command::try_from(args)?;

            match command {
                Command::Register(register) => {
                    let credential_id = app.register(register)?;
                    println!("{}", credential_id);
                }
                Command::Authenticate(authenticate) => {
                    let code = app.authenticate(authenticate)?;
                    println!("{}", code);
                }
                Command::Delete(label) => {
                    app.delete(label)?;
                }
                Command::List => app.list()?,
                Command::Reset => app.reset()?,
            }
        }

        // if let Some(args) = args.subcommand_matches("piv") {
        //     info!("interacting with PIV app");
        //     use solo2::apps::piv::App;
        //     if args.subcommand_matches("aid").is_some() {
        //         println!("{}", hex::encode(App::aid()).to_uppercase());
        //         return Ok(());
        //     }

        //     let mut app = App::new(uuid)?;
        //     app.select()?;
        // }

        if let Some(args) = args.subcommand_matches("provisioner") {
            info!("interacting with Provisioner app");
            use solo2::apps::provisioner::App;
            if args.subcommand_matches("aid").is_some() {
                println!("{}", hex::encode(App::aid()).to_uppercase());
                return Ok(());
            }

            let card = unwrap_or_interactively_select(uuid, "smartcards")?;
            let mut app = App::with(card);
            app.select()?;

            if args.subcommand_matches("generate-ed255-key").is_some() {
                let public_key = app.generate_trussed_ed255_attestation_key()?;
                println!("{}", hex::encode(public_key));
            }
            if args.subcommand_matches("generate-p256-key").is_some() {
                let public_key = app.generate_trussed_p256_attestation_key()?;
                println!("{}", hex::encode(public_key));
            }
            if args.subcommand_matches("generate-x255-key").is_some() {
                let public_key = app.generate_trussed_x255_attestation_key()?;
                println!("{}", hex::encode(public_key));
            }
            if args.subcommand_matches("reformat-filesystem").is_some() {
                app.reformat_filesystem()?;
            }
            if let Some(args) = args.subcommand_matches("store-ed255-cert") {
                let cert_file = args.value_of("DER").unwrap();
                let certificate = std::fs::read(cert_file)?;
                app.store_trussed_ed255_attestation_certificate(&certificate)?;
            }
            if let Some(args) = args.subcommand_matches("store-p256-cert") {
                let cert_file = args.value_of("DER").unwrap();
                let certificate = std::fs::read(cert_file)?;
                app.store_trussed_p256_attestation_certificate(&certificate)?;
            }
            if let Some(args) = args.subcommand_matches("store-x255-cert") {
                let cert_file = args.value_of("DER").unwrap();
                let certificate = std::fs::read(cert_file)?;
                app.store_trussed_x255_attestation_certificate(&certificate)?;
            }
            if let Some(args) = args.subcommand_matches("store-t1-pubkey") {
                let pubkey_file = args.value_of("BYTES").unwrap();
                let public_key: [u8; 32] = std::fs::read(pubkey_file)?.as_slice().try_into()?;
                app.store_trussed_t1_intermediate_public_key(public_key)?;
            }
            if args.subcommand_matches("boot-to-bootrom").is_some() {
                app.boot_to_bootrom()?;
            }
            if args.subcommand_matches("uuid").is_some() {
                let uuid = app.uuid()?;
                println!("{}", hex::encode_upper(uuid.to_be_bytes()));
            }
            if let Some(args) = args.subcommand_matches("write-file") {
                let file = args.value_of("DATA").unwrap();
                let data = std::fs::read(file)?;
                let path = args.value_of("PATH").unwrap();
                app.write_file(&data, path)?;
            }
        }

        if let Some(args) = args.subcommand_matches("tester") {
            info!("interacting with Tester app");
            use solo2::apps::tester::App;
            if args.subcommand_matches("aid").is_some() {
                println!("{}", hex::encode(App::aid()).to_uppercase());
                return Ok(());
            }

            let card = unwrap_or_interactively_select(uuid, "smartcards")?;
            let mut app = App::with(card);
            app.select()?;
        }
    }

    #[cfg(not(feature = "dev-pki"))]
    if args.subcommand_matches("dev-pki").is_some() {
        return Err(anyhow!(
            "Compile with `--features dev-pki` for dev PKI support!"
        ));
    }
    if let Some(args) = args.subcommand_matches("pki") {
        if let Some(args) = args.subcommand_matches("ca") {
            if let Some(args) = args.subcommand_matches("fetch-certificate") {
                use std::io::{stdout, Write as _};
                let authority: solo2::pki::Authority =
                    args.value_of("AUTHORITY").unwrap().try_into()?;
                let certificate = solo2::pki::fetch_certificate(authority)?;
                if atty::is(atty::Stream::Stdout) {
                    eprintln!("Some things to do with the DER data");
                    eprintln!(
                        "* redirect to a file: `> {:?}.der",
                        &authority.name().to_lowercase()
                    );
                    eprintln!("* inspect contents by piping to step: `| step certificate inspect");
                    return Err(anyhow::anyhow!("Refusing to write binary data to stdout"));
                }
                stdout().write_all(certificate.der())?;
            }
        }
        #[cfg(feature = "dev-pki")]
        if let Some(args) = args.subcommand_matches("dev") {
            if let Some(args) = args.subcommand_matches("fido") {
                let (aaguid, key_trussed, key_pem, cert) =
                    solo2::pki::dev::generate_selfsigned_fido();

                info!("\n{}", key_pem);
                info!("\n{}", cert.serialize_pem()?);

                std::fs::write(args.value_of("KEY").unwrap(), &key_trussed)?;
                std::fs::write(args.value_of("CERT").unwrap(), &cert.serialize_der()?)?;

                println!("{}", hex::encode_upper(aaguid));
            }
        }
    }

    if let Some(args) = args.subcommand_matches("bootloader") {
        if args.subcommand_matches("reboot").is_some() {
            let bootloader = match uuid {
                Some(uuid) => Bootloader::having(uuid)?,
                None => interactively_select(Bootloader::list(), "Solo 2 bootloaders")?,
            };
            bootloader.reboot();
        }
        if args.subcommand_matches("list").is_some() {
            let bootloaders = Bootloader::list();
            for bootloader in bootloaders {
                println!("{}", &Device::Bootloader(bootloader));
            }
        }
    }

    if let Some(_args) = args.subcommand_matches("list") {
        if !solo2::smartcard::Service::is_available() {
            return Err(anyhow::anyhow!("There is no PCSC service running"));
        }
        let devices = solo2::Device::list();
        for device in devices {
            println!("{}", &device);
        }
    }

    if let Some(args) = args.subcommand_matches("update") {
        if !solo2::smartcard::Service::is_available() {
            return Err(anyhow::anyhow!("There is no PCSC service running"));
        }
        let skip_major_prompt = args.is_present("yes");
        let update_all = args.is_present("all");

        let sb2_filepath = args.value_of("FIRMWARE").map(|s| s.to_string());

        use solo2::Firmware;

        let firmware: Firmware =
            sb2_filepath
                .map(Firmware::read_from_file)
                .unwrap_or_else(|| {
                    println!("Downloading latest release from https://github.com/solokeys/solo2/");
                    Firmware::download_latest()
                })?;

        println!("Fetched firmware version {}", &firmware.version().to_calver());

        if update_all {
            for device in Device::list() {
                device.program(firmware.clone(), skip_major_prompt)?;
            }
            return Ok(());
        } else {
            let device = match uuid {
                Some(uuid) => Device::having(uuid)?,
                None => interactively_select(Device::list(), "Solo 2 devices")?,
            };
            return device.program(firmware, skip_major_prompt);
        }
    }

    Ok(())
}

/// In `dialoguer` dialogs, the cursor is hidden and, if the user interrupts via Ctrl-C,
/// not shown again (for reasons). This is a best effort attempt to show the cursor again
/// in these situations.
fn restore_cursor_on_ctrl_c() {
    ctrlc::set_handler(move || {
        let term = dialoguer::console::Term::stderr();
        term.show_cursor().ok();
        // Ctrl-C exit code = 130
        std::process::exit(130);
    })
    .ok();
}
