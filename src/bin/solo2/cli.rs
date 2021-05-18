use clap::{self, crate_authors, crate_version, App, AppSettings, Arg, SubCommand};

const ABOUT: &str = "
solo2 is the go-to tool to interact with a Solo 2 security key.

Use -h for short descriptions and --help for more details

Project homepage: https://github.com/solokeys/solo2-cli
";
pub fn cli() -> clap::App<'static, 'static> {
    lazy_static::lazy_static! {
        static ref LONG_VERSION: String = long_version(None);
    }

    let cli = App::new("solo2")
        .author(crate_authors!())
        .version(crate_version!())
        .long_version(LONG_VERSION.as_str())
        .about(ABOUT)
        .help_message("Prints help information. Use --help for more details.")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::InferSubcommands)
        // apps
        .subcommand(
            SubCommand::with_name("app")
                .about("app interactions")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .setting(AppSettings::InferSubcommands)
                .subcommand(
                    SubCommand::with_name("mgmt")
                        .about("management app")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(SubCommand::with_name("aid").about("AID"))
                        .subcommand(
                            SubCommand::with_name("reboot").about("reboot device to regular mode"),
                        )
                        .subcommand(
                            SubCommand::with_name("boot-to-bootrom")
                                .about("reboot device to bootloader mode"),
                        )
                        .subcommand(SubCommand::with_name("uuid").about("UUID (serial number)"))
                        .subcommand(SubCommand::with_name("version").about("version")),
                )
                .subcommand(
                    SubCommand::with_name("ndef")
                        .about("NDEF app")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(SubCommand::with_name("aid").about("AID"))
                        .subcommand(
                            SubCommand::with_name("capabilities").about("NDEF capabilities"),
                        )
                        .subcommand(SubCommand::with_name("data").about("NDEF data")),
                )
                .subcommand(
                    SubCommand::with_name("provisioner")
                        .about("Provisioner app")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(SubCommand::with_name("aid").about("AID"))
                        .subcommand(
                            SubCommand::with_name("generate-ed255-key")
                                .about("Generate Trussed Ed255 attestation key"),
                        )
                        .subcommand(
                            SubCommand::with_name("generate-p256-key")
                                .about("Generate Trussed P256 attestation key"),
                        )
                        .subcommand(
                            SubCommand::with_name("store-ed255-cert")
                                .about("Store Trussed Ed255 attestation certificate")
                                .arg(
                                    Arg::with_name("DER")
                                        .help("Certificate in DER format")
                                        .required(true),
                                ),
                        )
                        .subcommand(
                            SubCommand::with_name("store-p256-cert")
                                .about("Store Trussed P256 attestation certificate")
                                .arg(
                                    Arg::with_name("DER")
                                        .help("Certificate in DER format")
                                        .required(true),
                                ),
                        )
                        .subcommand(
                            SubCommand::with_name("reformat-filesystem")
                                .about("Reformat internal filesystem"),
                        )
                        .subcommand(SubCommand::with_name("uuid").about("UUID (serial number)"))
                        .subcommand(
                            SubCommand::with_name("write-file")
                                .about("Write binary file to specified path")
                                .arg(
                                    Arg::with_name("DATA")
                                        .help("binary data file")
                                        .required(true),
                                )
                                .arg(
                                    Arg::with_name("PATH")
                                        .help("path in internal filesystem")
                                        .required(true),
                                ),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("piv")
                        .about("PIV (personal identity verification) app")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(SubCommand::with_name("aid").about("AID")),
                )
                .subcommand(
                    SubCommand::with_name("tester")
                        .about("Tester app")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(SubCommand::with_name("aid").about("AID")),
                ),
        )
        // dev PKI
        .subcommand(
            SubCommand::with_name("dev-pki")
                .about("PKI for development")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .setting(AppSettings::InferSubcommands)
                .subcommand(
                    SubCommand::with_name("fido")
                        .about("generate a self-signed FIDO batch attestation cert+key")
                        .arg(
                            Arg::with_name("KEY")
                                .help("output file, for private P256 key in binary format")
                                .required(true),
                        )
                        .arg(
                            Arg::with_name("CERT")
                                .help("output file, for self-signed certificate in DER format")
                                .required(true),
                        ),
                ),
        )
        // inherited from lpc55-host
        .subcommand(
            SubCommand::with_name("provision")
                // .version(crate_version!())
                // .long_version(LONG_VERSION.as_str())
                .about("Run a sequence of bootloader commands defined in the config file.")
                .arg(
                    Arg::with_name("CONFIG")
                        .help("Configuration file containing settings")
                        .required(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("bootloader")
                .about("Interact with bootloader")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .setting(AppSettings::InferSubcommands)
                .subcommand(
                    SubCommand::with_name("reboot")
                        .about("reboot (into device if firmware is valid)"),
                )
                .subcommand(SubCommand::with_name("ls").about("list all available bootloaders")),
        );

    cli
}

/// Return the "long" format of lpc55's version string.
///
/// If a revision hash is given, then it is used. If one isn't given, then
/// the SOLO2_CLI_BUILD_GIT_HASH env var is inspected for it. If that isn't set,
/// then a revision hash is not included in the version string returned.
pub fn long_version(revision_hash: Option<&str>) -> String {
    // Do we have a git hash?
    // (Yes, if ripgrep was built on a machine with `git` installed.)
    let hash = match revision_hash.or(option_env!("SOLO2_CLI_BUILD_GIT_HASH")) {
        None => String::new(),
        Some(githash) => format!(" (rev {})", githash),
    };
    format!("{}{}", crate_version!(), hash)
}
