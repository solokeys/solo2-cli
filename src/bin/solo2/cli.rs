use clap::{self, crate_authors, crate_version, App, AppSettings, Arg, SubCommand};

const ABOUT: &str = "
solo2 is the go-to tool to interact with a Solo 2 security key.

Use -h for short descriptions and --help for more details.

Print more logs by setting env SOLO2_LOG='info' or SOLO2_LOG='debug'.

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
        .arg(Arg::with_name("uuid")
                .long("uuid")
                .short("u")
                .help("Specify a 16 byte UUID for a Solo 2 / Trussed compatible device to connect to.")
                .value_name("UUID")
        )
        // apps
        .subcommand(
            SubCommand::with_name("app")
                .about("app interactions")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .setting(AppSettings::InferSubcommands)
                .subcommand(
                    SubCommand::with_name("admin")
                        .about("admin app")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(SubCommand::with_name("aid").about("Prints the application's AID"))
                        .subcommand(
                            SubCommand::with_name("reboot").about("Reboots device to regular mode"),
                        )
                        .subcommand(
                            SubCommand::with_name("boot-to-bootrom")
                                .about("Reboots device to bootloader mode"),
                        )
                        .subcommand(SubCommand::with_name("uuid").about("UUID (serial number)"))
                        .subcommand(SubCommand::with_name("version").about("version")),
                )
                .subcommand(
                    SubCommand::with_name("ndef")
                        .about("NDEF app")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(SubCommand::with_name("aid").about("Prints the application's AID"))
                        .subcommand(
                            SubCommand::with_name("capabilities").about("NDEF capabilities"),
                        )
                        .subcommand(SubCommand::with_name("data").about("NDEF data")),
                )
                // .subcommand(
                //     SubCommand::with_name("oath")
                //         .about("OATH app")
                //         .setting(AppSettings::SubcommandRequiredElseHelp)
                //         .setting(AppSettings::InferSubcommands)
                //         .subcommand(SubCommand::with_name("aid").about("Prints the application's AID"))
                //         .subcommand(SubCommand::with_name("register")
                //             .about("Registers an OATH secret")
                //             // FYI, imf = OATH initial moving factor
                //             .arg(Arg::with_name("algorithm")
                //                  .long("algorithm")
                //                  .short("a")
                //                  .help("hash algorithm to use in OTP generation")
                //                  .value_name("DIGEST")
                //                  .default_value("SHA1")
                //                  .possible_values(&["SHA1", "SHA256"])
                //                  .case_insensitive(true)
                //              )
                //             .arg(Arg::with_name("counter")
                //                  .long("counter")
                //                  .short("c")
                //                  .help("(only HOTP) initial counter to use for HOTPs")
                //                  .value_name("INITIAL-COUNTER")
                //                  .default_value("0")
                //                  .required_if("kind", "HOTP")
                //              )
                //             .arg(Arg::with_name("digits")
                //                  .long("digits")
                //                  .short("d")
                //                  .help("number of digits to use for the OATH OTP values")
                //                  .value_name("ISSUER")
                //                  .default_value("6")
                //                  .possible_values(&["6", "7", "8"])
                //              )
                //             .arg(Arg::with_name("issuer")
                //                  .long("issuer")
                //                  .short("i")
                //                  .help("(optional) issuer to use for the OATH credential, e.g., example.com")
                //                  .value_name("ISSUER")
                //              )
                //             .arg(Arg::with_name("kind")
                //                  .long("kind")
                //                  .short("k")
                //                  // for compatibility with `ykman`
                //                  .aliases(&["o", "oath-type"])
                //                  .help("kind of OATH credential to register")
                //                  .value_name("KIND")
                //                  .default_value("TOTP")
                //                  .case_insensitive(true)
                //                  .possible_values(&["HOTP", "TOTP"])
                //              )
                //             .arg(Arg::with_name("period")
                //                  .long("period")
                //                  .short("p")
                //                  .help("(only TOTP) period in seconds for which a TOTP should be valid")
                //                  .value_name("PERIOD")
                //                  .default_value("30")
                //                  .required_if("kind", "TOTP")
                //              )
                //             .arg(Arg::with_name("label")
                //                  .help("label to use for the OATH secret, e.g. alice@trussed.dev")
                //                  .value_name("LABEL")
                //                  .required(true)
                //              )
                //             .arg(Arg::with_name("secret")
                //                  .help("the actual TOTP seed, e.g. JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP")
                //                  .value_name("SECRET")
                //                  .required(true)
                //              )
                //             .arg(Arg::with_name("sha1")
                //                  .long("sha1")
                //                  .help("use SHA1 hash algorithm in OTP generation")
                //                  .conflicts_with_all(&["sha256", "algorithm"])
                //              )
                //             .arg(Arg::with_name("sha256")
                //                  .long("sha256")
                //                  .help("use SHA256 hash algorithm in OTP generation")
                //                  .conflicts_with_all(&["sha1", "algorithm"])
                //              )
                //             .arg(Arg::with_name("hotp")
                //                  .long("hotp")
                //                  .help("generate HOTP flavor of OATH credentials.")
                //                  .conflicts_with_all(&["totp", "kind"])
                //              )
                //             .arg(Arg::with_name("totp")
                //                  .long("totp")
                //                  .help("generate TOTP flavor of OATH credentials.")
                //                  .conflicts_with_all(&["hotp", "kind"])
                //              )
                //         )
                //         .subcommand(SubCommand::with_name("totp")
                //             .about("Generates a TOTP from a previously registered credential")
                //             .arg(Arg::with_name("TIMESTAMP")
                //                  .short("t")
                //                  .long("timestamp")
                //                  .hidden(true)
                //                  .help("timestamp to use to generate the OTP, as seconds since the UNIX epoch")
                //                  .value_name("TIMESTAMP")
                //                  .required(false)
                //              )
                //             .arg(Arg::with_name("label")
                //                  .help("label of the TOTP credential to use, e.g. alice@trussed.dev")
                //                  .value_name("LABEL")
                //                  .required(true)
                //              )
                //         )
                //         .subcommand(SubCommand::with_name("delete")
                //             .about("Deletes a previously registered credential")
                //             .arg(Arg::with_name("label")
                //                  .help("label of the TOTP credential to delete, e.g. alice@trussed.dev")
                //                  .value_name("LABEL")
                //                  .required(true)
                //              )
                //         )
                //         .subcommand(SubCommand::with_name("list")
                //             .about("Lists credentials stored on the device")
                //             .visible_alias("ls")
                //         )
                //         .subcommand(SubCommand::with_name("reset")
                //             .about("Reset device, deleting all credentials")
                //         )
                // )
                .subcommand(
                    SubCommand::with_name("provisioner")
                        .about("Provisioner app")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(SubCommand::with_name("aid").about("Prints the application's AID"))
                        .subcommand(
                            SubCommand::with_name("generate-ed255-key")
                                .about("Generates a new Trussed Ed255 attestation key"),
                        )
                        .subcommand(
                            SubCommand::with_name("generate-p256-key")
                                .about("Generates a new Trussed P256 attestation key"),
                        )
                        .subcommand(
                            SubCommand::with_name("generate-x255-key")
                                .about("Generates a new Trussed X255 agreement key"),
                        )
                        .subcommand(
                            SubCommand::with_name("store-ed255-cert")
                                .about("Stores a Trussed Ed255 attestation certificate")
                                .arg(
                                    Arg::with_name("DER")
                                        .help("Certificate in DER format")
                                        .required(true),
                                ),
                        )
                        .subcommand(
                            SubCommand::with_name("store-p256-cert")
                                .about("Stores a Trussed P256 attestation certificate")
                                .arg(
                                    Arg::with_name("DER")
                                        .help("Certificate in DER format")
                                        .required(true),
                                ),
                        )
                        .subcommand(
                            SubCommand::with_name("store-x255-cert")
                                .about("Stores a Trussed X255 attestation certificate")
                                .arg(
                                    Arg::with_name("DER")
                                        .help("Certificate in DER format")
                                        .required(true),
                                ),
                        )
                        .subcommand(
                            SubCommand::with_name("store-t1-pubkey")
                                .about("Stores the Trussed T1 intermediate public key")
                                .arg(
                                    Arg::with_name("BYTES")
                                        .help("Ed255 public key (raw, 32B)")
                                        .required(true),
                                ),
                        )
                        .subcommand(
                            SubCommand::with_name("reformat-filesystem")
                                .about("Reformats the internal filesystem"),
                        )
                        .subcommand(SubCommand::with_name("boot-to-bootrom").about("Boot to ROM bootloader"))
                        .subcommand(SubCommand::with_name("uuid").about("UUID (serial number)"))
                        .subcommand(
                            SubCommand::with_name("write-file")
                                .about("Writes binary file to specified path")
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
                        .subcommand(SubCommand::with_name("aid").about("Prints the application's AID")),
                )
                .subcommand(
                    SubCommand::with_name("tester")
                        .about("Tester app")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(SubCommand::with_name("aid").about("Prints the application's AID")),
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
                        .about("Generates a self-signed FIDO batch attestation cert+key")
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
                .about("Runs a sequence of bootloader commands defined in the config file.")
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
                        .about("Reboots (into device if firmware is valid)"),
                )
                .subcommand(SubCommand::with_name("ls").about("Lists all available bootloaders")),
        )
        .subcommand(
            SubCommand::with_name("update")
                .about("Update to latest firmware published by SoloKeys.  Warns on Major updates.")
                .arg(
                    Arg::with_name("yes")
                        .short("y")
                        .long("yes")
                        .help("Proceed with major updates without prompt.")
                        .required(false),
                )
                .arg(
                    Arg::with_name("all")
                        .short("a")
                        .long("all")
                        .help("Update all connect Solo devices.")
                        .required(false),
                )
                .arg(
                    Arg::with_name("FIRMWARE")
                        .help("Update to a specific firmware secure boot file (.sb2).")
                        .required(false),
                ),

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
