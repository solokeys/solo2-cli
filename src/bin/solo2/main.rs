#[macro_use]
extern crate log;

mod cli;

// use core::convert::TryFrom;

use anyhow::anyhow;
use lpc55::bootloader::Bootloader;

use solo2;

#[tokio::main]
async fn main() {
    pretty_env_logger::init_custom_env("SOLO2_LOG");
    info!("solo2 CLI startup");

    let args = cli::cli().get_matches();

    if let Err(err) = try_main(args).await {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

async fn try_main(args: clap::ArgMatches<'_>) -> anyhow::Result<()> {

    let uuid_vec_maybe = args.value_of("uuid").map(|uuid| hex::decode(uuid).unwrap());
    let uuid = if let Some(uuid_vec) = uuid_vec_maybe {
        if uuid_vec.len() != 16 {
            return Err(anyhow::anyhow!("UUID must be 16 bytes."));
        }

        let mut uuid = [0u8; 16];
        uuid.copy_from_slice(&uuid_vec);

        Some(uuid)
    } else {
        None
    };

    if let Some(args) = args.subcommand_matches("app") {
        use solo2::apps::App;

        if let Some(args) = args.subcommand_matches("admin") {
            info!("interacting with admin app");
            use solo2::apps::admin::App as AdminApp;
            if args.subcommand_matches("aid").is_some() {
                println!("{}", hex::encode(AdminApp::aid()).to_uppercase());
                return Ok(());
            }

            let mut app = AdminApp::new(uuid)?;
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
                println!("{}", hex::encode_upper(uuid.to_be_bytes()));
            }
            if args.subcommand_matches("version").is_some() {
                let version = app.version()?;
                println!("{}", version);
            }
        }

        if let Some(args) = args.subcommand_matches("ndef") {
            info!("interacting with NDEF app");
            use solo2::apps::ndef::App as NdefApp;
            if args.subcommand_matches("aid").is_some() {
                println!("{}", hex::encode(NdefApp::aid()).to_uppercase());
                return Ok(());
            }

            let mut app = NdefApp::new(uuid)?;
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

        // if let Some(args) = args.subcommand_matches("oath") {
        //     info!("interacting with OATH app");
        //     use solo2::apps::oath::{App, Command};
        //     if args.subcommand_matches("aid").is_some() {
        //         App::print_aid();
        //         return Ok(());
        //     }

        //     let mut app = App::new(uuid)?;
        //     app.select()?;

        //     let command: Command = Command::try_from(args)?;

        //     match command {
        //         Command::Register(register) => {
        //             let credential_id = app.register(register)?;
        //             println!("{}", credential_id);
        //         }
        //         Command::Authenticate(authenticate) => {
        //             let code = app.authenticate(authenticate)?;
        //             println!("{}", code);
        //         }
        //         Command::Delete(label) => {
        //             app.delete(label)?;
        //         }
        //         Command::List => app.list()?,
        //         Command::Reset => app.reset()?,
        //     }
        // }

        if let Some(args) = args.subcommand_matches("piv") {
            info!("interacting with PIV app");
            use solo2::apps::piv::App;
            if args.subcommand_matches("aid").is_some() {
                println!("{}", hex::encode(App::aid()).to_uppercase());
                return Ok(());
            }

            let mut app = App::new(uuid)?;
            app.select()?;
        }

        if let Some(args) = args.subcommand_matches("provisioner") {
            info!("interacting with Provisioner app");
            use solo2::apps::provisioner::App;
            if args.subcommand_matches("aid").is_some() {
                println!("{}", hex::encode(App::aid()).to_uppercase());
                return Ok(());
            }

            let mut app = App::new(uuid)?;
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
                let public_key: [u8; 32] = std::fs::read(pubkey_file)?
                    .as_slice()
                    .try_into()?;
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
                app.write_file(&data, &path)?;
            }
        }

        if let Some(args) = args.subcommand_matches("tester") {
            info!("interacting with Tester app");
            use solo2::apps::tester::App;
            if args.subcommand_matches("aid").is_some() {
                println!("{}", hex::encode(App::aid()).to_uppercase());
                return Ok(());
            }

            let mut app = App::new(uuid)?;
            app.select()?;
        }
    }

    #[cfg(not(feature = "dev-pki"))]
    if args.subcommand_matches("dev-pki").is_some() {
        return Err(anyhow!(
            "Compile with `--features dev-pki` for dev PKI support!"
        ));
    }
    #[cfg(feature = "dev-pki")]
    if let Some(args) = args.subcommand_matches("dev-pki") {
        if let Some(args) = args.subcommand_matches("fido") {
            let (aaguid, key_trussed, key_pem, cert) = solo2::dev_pki::generate_selfsigned_fido();

            info!("\n{}", key_pem);
            info!("\n{}", cert.serialize_pem()?);

            std::fs::write(args.value_of("KEY").unwrap(), &key_trussed)?;
            std::fs::write(args.value_of("CERT").unwrap(), &cert.serialize_der()?)?;

            println!("{}", hex::encode_upper(aaguid));
        }
    }

    if let Some(args) = args.subcommand_matches("bootloader") {

        if args.subcommand_matches("reboot").is_some() {
            let bootloader = solo2::device_selection::find_bootloader(uuid)?;
            bootloader.reboot();
        }
        if args.subcommand_matches("ls").is_some() {
            let bootloaders = Bootloader::list();
            for bootloader in bootloaders {
                println!("{:?}", &bootloader);
            }
        }
    }

    if let Some(args) = args.subcommand_matches("update") {
        let skip_major_check = args.is_present("yes");
        let update_all = args.is_present("all");

        let sb2file = args.value_of("FIRMWARE").map(|s| s.to_string());
        solo2::update::run_update_procedure(sb2file, uuid, skip_major_check, update_all).await?;
    }

    Ok(())
}
