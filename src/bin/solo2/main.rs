#[macro_use]
extern crate log;

mod cli;

use anyhow::anyhow;
use solo2::{Device, Select as _, Solo2, Uuid, UuidSelectable};

fn main() {
    restore_cursor_on_ctrl_c();

    use clap::Parser;
    let args = cli::Cli::parse();

    use log::LevelFilter::*;
    let level = match args.global_options.verbose {
        0 => Warn,
        1 => Info,
        2 => Debug,
        _ => Trace,
    };
    pretty_env_logger::formatted_builder()
        .filter(None, level)
        .init();

    if let Err(err) = try_main(args) {
        eprintln!("Error: {}", err);
        std::process::exit(1);
    }
}

fn try_main(args: cli::Cli) -> anyhow::Result<()> {
    let uuid: Option<Uuid> = args
        .global_options
        .uuid
        .map(|uuid| uuid.parse())
        .transpose()?;

    if args.global_options.ctap {
        Solo2::prefer_ctap();
    }
    if args.global_options.pcsc {
        Solo2::prefer_pcsc();
    }

    // use cli::Subcommands::*;
    use cli::Apps::*;
    match args.subcommand {
        cli::Subcommands::App(app) => {
            let solo2s: Vec<Solo2> =
                all_or_unwrap_or_interactively_select(uuid, args.global_options.all, "Solo 2")?;

            // let uuid = solo2.uuid();

            solo2s.into_iter().try_for_each(|mut solo2| {
                match &app {
                    Admin(admin) => {
                        use cli::Admin::*;
                        use solo2::apps::Admin;

                        let mut app = Admin::select(&mut solo2)?;

                        match admin {
                            Aid => {
                                println!("{}", hex::encode(Admin::application_id()).to_uppercase());
                                return Ok(());
                            }
                            Locked => {
                                let locked = app.locked()?;
                                println!("locked: {}", locked);
                            }
                            Restart => {
                                info!("attempting restart of devices");
                                app.reboot()?;
                            }
                            Maintenance => {
                                drop(app);
                                println!("Tap button on key to reboot into bootloader/maintenance mode, or replug to abort...");
                                solo2.into_lpc55()?;
                            }
                            Uuid => {
                                let uuid = app.uuid()?;
                                println!("{:X}", uuid.to_simple());
                            }
                            Version => {
                                let version = app.version()?;
                                println!("{}", version.to_calver());
                            }
                            Wink => {
                                app.wink()?;
                            }
                        }
                        Ok(())
                    }
                    Fido(fido) => {
                        use cli::Fido::*;
                        use solo2::apps::Fido;

                        let app = Fido::from(solo2.as_ctap_mut().ok_or_else(|| anyhow!("CTAP unavailable"))?);

                        match fido {
                            Init => {
                                println!("{:?}", app.init()?);
                            }
                            Wink => {
                                let channel = app.init()?.channel;
                                app.wink(channel)?;
                            }
                        }
                        Ok(())
                    }
                    Ndef(ndef) => {
                        use cli::Ndef::*;
                        use solo2::apps::Ndef;

                        let mut app = Ndef::select(&mut solo2)?;

                        match ndef {
                            Aid => {
                                println!("{}", hex::encode(Ndef::application_id()).to_uppercase());
                                return Ok(());
                            }
                            Capabilities => {
                                let capabilities = app.capabilities()?;
                                println!("{}", hex::encode(capabilities));
                            }
                            Data => {
                                let data = app.data()?;
                                println!("{}", hex::encode(data));
                            }
                        }
                        Ok(())
                    }
                    Oath(oath) => {
                        use cli::Oath::*;
                        use solo2::apps::Oath;

                        let mut app = Oath::select(&mut solo2)?;

                        match oath {
                            Aid => {
                                println!("{}", hex::encode(Oath::application_id()).to_uppercase());
                                Ok(())
                            }
                            Delete { label } => {
                                app.delete(label.clone())?;
                                Ok(())
                            }
                            List => {
                                let labels = app.list()?;
                                for label in labels {
                                    println!("{}", label);
                                }
                                Ok(())
                            }
                            // TODO: factor out the conversion
                            Register(args) => {
                                use solo2::apps::oath;

                                use cli::OathAlgorithm;
                                let digest = match args.algorithm {
                                    OathAlgorithm::Sha1 => oath::Digest::Sha1,
                                    OathAlgorithm::Sha256 => oath::Digest::Sha256,
                                };
                                let secret =
                                    solo2::apps::oath::Secret::from_base32(&args.secret.to_uppercase(), digest)?;
                                use cli::OathKind;
                                let kind = match args.kind {
                                    OathKind::Hotp => oath::Kind::Hotp(oath::Hotp {
                                        initial_counter: args.counter,
                                    }),
                                    OathKind::Totp => oath::Kind::Totp(oath::Totp {
                                        period: args.period,
                                    }),
                                };
                                let credential = solo2::apps::oath::Credential {
                                    label: args.label.clone(),
                                    issuer: args.issuer.clone(),
                                    secret,
                                    kind,
                                    algorithm: digest,
                                    digits: args.digits,
                                };
                                let credential_id = app.register(credential)?;
                                println!("{}", credential_id);
                                Ok(())
                            }
                            Reset => app.reset(),
                            // TODO: factor out the conversion
                            Totp { label, timestamp } => {
                                use solo2::apps::oath;
                                use std::time::SystemTime;

                                let timestamp = timestamp
                                    .clone()
                                    .map(|s| s.parse())
                                    .transpose()?
                                    .unwrap_or_else(|| {
                                        let since_epoch = SystemTime::now()
                                            .duration_since(SystemTime::UNIX_EPOCH)
                                            .unwrap();
                                        since_epoch.as_secs()
                                    });
                                let authenticate = oath::Authenticate { label: label.clone(), timestamp };
                                let code = app.authenticate(authenticate)?;
                                println!("{}", code);
                                Ok(())
                            }
                        }
                    }
                    Piv(piv) => {
                        use cli::Piv::*;
                        use solo2::apps::Piv;

                        // let mut app = Piv::select(&mut solo2)?;
                        Piv::select(&mut solo2)?;

                        match piv {
                            Aid => {
                                println!("{}", hex::encode(Piv::application_id()).to_uppercase());
                                Ok(())
                            }
                        }
                    }
                    Provision(provision) => {
                        use cli::Provision::*;
                        use solo2::apps::provision::App as Provision;

                        let mut app = Provision::select(&mut solo2)?;

                        match provision {
                            Aid => {
                                println!(
                                    "{}",
                                    hex::encode(Provision::application_id()).to_uppercase()
                                );
                                return Ok(());
                            }
                            GenerateEd255Key => {
                                let public_key = app.generate_trussed_ed255_attestation_key()?;
                                println!("{}", hex::encode(public_key));
                            }
                            GenerateP256Key => {
                                let public_key = app.generate_trussed_p256_attestation_key()?;
                                println!("{}", hex::encode(public_key));
                            }
                            GenerateX255Key => {
                                let public_key = app.generate_trussed_x255_attestation_key()?;
                                println!("{}", hex::encode(public_key));
                            }
                            ReformatFilesystem => app.reformat_filesystem()?,
                            StoreEd255Cert { der } => {
                                let certificate = std::fs::read(der)?;
                                app.store_trussed_ed255_attestation_certificate(&certificate)?;
                            }
                            StoreP256Cert { der } => {
                                let certificate = std::fs::read(der)?;
                                app.store_trussed_p256_attestation_certificate(&certificate)?;
                            }
                            StoreX255Cert { der } => {
                                let certificate = std::fs::read(der)?;
                                app.store_trussed_x255_attestation_certificate(&certificate)?;
                            }
                            StoreT1Pubkey { bytes } => {
                                let pubkey: [u8; 32] = std::fs::read(bytes)?.as_slice().try_into()?;
                                app.store_trussed_t1_intermediate_public_key(pubkey)?;
                            }
                            StoreFidoBatchCert { cert } => {
                                let cert = std::fs::read(cert)?;
                                app.write_file(&cert, "/fido/x5c/00")?;
                            }
                            StoreFidoBatchKey { bytes } => {
                                let key = std::fs::read(bytes)?;
                                app.write_file(&key, "/fido/sec/00")?;
                            }
                            WriteFile { data, path } => {
                                let data = std::fs::read(data)?;
                                app.write_file(&data, path)?;
                            }
                        }
                        Ok(())
                    }
                    Qa(cmd) => {
                        use cli::Qa::*;
                        use solo2::apps::qa::App;

                        App::select(&mut solo2)?;

                        match cmd {
                            Aid => {
                                println!("{}", hex::encode(App::application_id()).to_uppercase());
                            }
                        }
                        Ok(())
                    }
                }
            })?;
        }
        cli::Subcommands::Pki(pki) => {
            match pki {
                cli::Pki::Ca(ca) => match ca {
                    cli::Ca::FetchCertificate { authority } => {
                        use std::io::{stdout, Write as _};
                        let authority: solo2::pki::Authority = authority.as_str().try_into()?;
                        let certificate = solo2::pki::fetch_certificate(authority)?;
                        if atty::is(atty::Stream::Stdout) {
                            eprintln!("Some things to do with the DER data");
                            eprintln!(
                                "* redirect to a file: `> {}.der`",
                                &authority.name().to_lowercase()
                            );
                            eprintln!("* inspect contents by piping to step: `| step certificate inspect`");
                            return Err(anyhow::anyhow!("Refusing to write binary data to stdout"));
                        }
                        stdout().write_all(certificate.der())?;
                    }
                },
                #[cfg(feature = "dev-pki")]
                cli::Pki::Dev(dev) => match dev {
                    cli::Dev::Fido { key, cert } => {
                        let (aaguid, key_trussed, key_pem, certificate) =
                            solo2::pki::dev::generate_selfsigned_fido();

                        info!("\n{}", key_pem);
                        info!("\n{}", certificate.serialize_pem()?);

                        std::fs::write(key, &key_trussed)?;
                        std::fs::write(cert, &certificate.serialize_der()?)?;

                        println!("{}", hex::encode_upper(aaguid));
                    }
                },
                cli::Pki::Web => {
                    let solo2: Solo2 = unwrap_or_interactively_select(uuid, "Solo 2")?;
                    let uuid = solo2.uuid().to_simple();
                    let url = format!("https://s2pki.net/s2/{uuid}/x255.txt");
                    println!("=> {}", url);
                    webbrowser::open(&url)?;
                }
            }
        }
        cli::Subcommands::Bootloader(args) => match args {
            cli::Bootloader::Reboot => {
                let bootloader = match uuid {
                    Some(uuid) => lpc55::Bootloader::having(uuid)?,
                    None => interactively_select(lpc55::Bootloader::list(), "Solo 2 bootloaders")?,
                };
                bootloader.reboot();
            }
            cli::Bootloader::List => {
                let bootloaders = lpc55::Bootloader::list();
                for bootloader in bootloaders {
                    println!("{}", &Device::Lpc55(bootloader));
                }
            }
        },
        cli::Subcommands::Completion(args) => {
            use clap::CommandFactory as _;
            use clap_complete::{generate, shells::*};
            use std::io::stdout;
            let mut app = cli::Cli::command();
            match args {
                cli::Completion::Bash => generate(Bash, &mut app, "solo2", &mut stdout()),
                cli::Completion::Fish => generate(Fish, &mut app, "solo2", &mut stdout()),
                cli::Completion::PowerShell => {
                    generate(PowerShell, &mut app, "solo2", &mut stdout())
                }
                cli::Completion::Zsh => generate(Zsh, &mut app, "solo2", &mut stdout()),
            }
        }
        cli::Subcommands::List => {
            let devices = solo2::Device::list();
            for device in devices {
                println!("{}", &device);
            }
        }
        cli::Subcommands::Update {
            dry_run,
            yes,
            all,
            with,
        } => {
            let firmware: solo2::Firmware = with
                .map(solo2::Firmware::read_from_file)
                .unwrap_or_else(|| {
                    println!("Downloading latest release from https://github.com/solokeys/solo2/");
                    solo2::Firmware::download_latest()
                })?;

            println!(
                "Fetched firmware version {} ({})",
                &firmware.version().to_calver(),
                &firmware.version().to_semver(),
            );

            if dry_run {
                return Ok(());
            }

            if all {
                for device in Device::list() {
                    device.program(firmware.clone(), yes)?;
                }
                return Ok(());
            } else {
                let device = match uuid {
                    Some(uuid) => Device::having(uuid)?,
                    None => interactively_select(Device::list(), "Solo 2 devices")?,
                };
                return device.program(firmware, yes);
            }
        }
    }

    Ok(())
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

pub fn all_or_unwrap_or_interactively_select<T: core::fmt::Display + UuidSelectable>(
    uuid: Option<Uuid>,
    all: bool,
    description: &str,
) -> anyhow::Result<Vec<T>> {
    let thing = match uuid {
        Some(uuid) => vec![T::having(uuid)?],
        None => match all {
            true => T::list(),
            false => vec![interactively_select(T::list(), description)?],
        },
    };
    Ok(thing)
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
