use clap::{self, crate_authors, crate_version, App, AppSettings, Arg};

const ABOUT: &str = "
solo2 is the go-to tool to interact with a Solo 2 security key.
Print more logs by setting env SOLO2_LOG='info' or SOLO2_LOG='debug'.

Project homepage: https://github.com/solokeys/solo2-cli
";
pub fn cli() -> clap::App<'static> {
    lazy_static::lazy_static! {
        static ref LONG_VERSION: String = long_version(None);
    }

    let cli = App::new("solo2")
        .author(crate_authors!())
        .version(crate_version!())
        .long_version(LONG_VERSION.as_str())
        .about(ABOUT)
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::InferSubcommands)
        .arg(
            Arg::new("ctap")
                .long("ctap")
                .takes_value(false)
                .global(true)
                .help("Prefer CTAP transport.")
                .help_heading("TRANSPORT")
        )
        .arg(
            Arg::new("pcsc")
                .long("pcsc")
                .takes_value(false)
                .global(true)
                .help("Prefer PCSC transport.")
                .help_heading("TRANSPORT")
        )
        .arg(Arg::new("uuid")
                .long("uuid")
                .short('u')
                .help("Specify a 16 byte UUID for a Solo 2 / Trussed compatible device to connect to.")
                .help_heading("SELECTION")
                .value_name("UUID")
        )
        // apps
        .subcommand(
            App::new("app")
                .about("app interactions")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .setting(AppSettings::InferSubcommands)
                .subcommand(
                    App::new("admin")
                        .about("admin app")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(App::new("aid").about("Prints the application's AID"))
                        .subcommand(
                            App::new("reboot").about("Reboots device to regular mode"),
                        )
                        .subcommand(
                            App::new("boot-to-bootrom")
                                .about("Reboots device to bootloader mode"),
                        )
                        .subcommand(App::new("uuid").about("UUID (serial number)"))
                        .subcommand(App::new("version").about("version")),
                )
                .subcommand(
                    App::new("fido")
                        .about("FIDO app")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(App::new("aid").about("Prints the application's AID"))
                        .subcommand(App::new("init").about("FIDO init reponse"))
                        .subcommand(App::new("wink").about("FIDO wink"))
                )
                .subcommand(
                    App::new("ndef")
                        .about("NDEF app")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(App::new("aid").about("Prints the application's AID"))
                        .subcommand(
                            App::new("capabilities").about("NDEF capabilities"),
                        )
                        .subcommand(App::new("data").about("NDEF data")),
                )
                .subcommand(
                    App::new("oath")
                        .about("OATH app")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(App::new("aid").about("Prints the application's AID"))
                        .subcommand(App::new("register")
                            .about("Registers an OATH secret")
                            // FYI, imf = OATH initial moving factor
                            .arg(Arg::new("algorithm")
                                 .long("algorithm")
                                 .short('a')
                                 .help("hash algorithm to use in OTP generation")
                                 .value_name("DIGEST")
                                 .default_value("SHA1")
                                 .possible_values(&["SHA1", "SHA256"])
                                 .ignore_case(true)
                             )
                            .arg(Arg::new("counter")
                                 .long("counter")
                                 .short('c')
                                 .help("(only HOTP) initial counter to use for HOTPs")
                                 .value_name("INITIAL-COUNTER")
                                 .default_value("0")
                                 .required_if_eq("kind", "HOTP")
                             )
                            .arg(Arg::new("digits")
                                 .long("digits")
                                 .short('d')
                                 .help("number of digits to use for the OATH OTP values")
                                 .value_name("ISSUER")
                                 .default_value("6")
                                 .possible_values(&["6", "7", "8"])
                             )
                            .arg(Arg::new("issuer")
                                 .long("issuer")
                                 .short('i')
                                 .help("(optional) issuer to use for the OATH credential, e.g., example.com")
                                 .value_name("ISSUER")
                             )
                            .arg(Arg::new("kind")
                                 .long("kind")
                                 .short('k')
                                 // for compatibility with `ykman`
                                 .aliases(&["o", "oath-type"])
                                 .help("kind of OATH credential to register")
                                 .value_name("KIND")
                                 .default_value("TOTP")
                                 .ignore_case(true)
                                 .possible_values(&["HOTP", "TOTP"])
                             )
                            .arg(Arg::new("period")
                                 .long("period")
                                 .short('p')
                                 .help("(only TOTP) period in seconds for which a TOTP should be valid")
                                 .value_name("PERIOD")
                                 .default_value("30")
                                 .required_if_eq("kind", "TOTP")
                             )
                            .arg(Arg::new("label")
                                 .help("label to use for the OATH secret, e.g. alice@trussed.dev")
                                 .value_name("LABEL")
                                 .required(true)
                             )
                            .arg(Arg::new("secret")
                                 .help("the actual TOTP seed, e.g. JBSWY3DPEHPK3PXPJBSWY3DPEHPK3PXP")
                                 .value_name("SECRET")
                                 .required(true)
                             )
                            .arg(Arg::new("sha1")
                                 .long("sha1")
                                 .help("use SHA1 hash algorithm in OTP generation")
                                 .conflicts_with_all(&["sha256", "algorithm"])
                             )
                            .arg(Arg::new("sha256")
                                 .long("sha256")
                                 .help("use SHA256 hash algorithm in OTP generation")
                                 .conflicts_with_all(&["sha1", "algorithm"])
                             )
                            .arg(Arg::new("hotp")
                                 .long("hotp")
                                 .help("generate HOTP flavor of OATH credentials.")
                                 .conflicts_with_all(&["totp", "kind"])
                             )
                            .arg(Arg::new("totp")
                                 .long("totp")
                                 .help("generate TOTP flavor of OATH credentials.")
                                 .conflicts_with_all(&["hotp", "kind"])
                             )
                        )
                        .subcommand(App::new("totp")
                            .about("Generates a TOTP from a previously registered credential")
                            .arg(Arg::new("TIMESTAMP")
                                 .short('t')
                                 .long("timestamp")
                                 .hide(true)
                                 .help("timestamp to use to generate the OTP, as seconds since the UNIX epoch")
                                 .value_name("TIMESTAMP")
                                 .required(false)
                             )
                            .arg(Arg::new("label")
                                 .help("label of the TOTP credential to use, e.g. alice@trussed.dev")
                                 .value_name("LABEL")
                                 .required(true)
                             )
                        )
                        .subcommand(App::new("delete")
                            .about("Deletes a previously registered credential")
                            .arg(Arg::new("label")
                                 .help("label of the TOTP credential to delete, e.g. alice@trussed.dev")
                                 .value_name("LABEL")
                                 .required(true)
                             )
                        )
                        .subcommand(App::new("list")
                            .about("Lists credentials stored on the device")
                            .visible_alias("ls")
                        )
                        .subcommand(App::new("reset")
                            .about("Reset device, deleting all credentials")
                        )
                )
                .subcommand(
                    App::new("provision")
                        .about("app for initial provisioning of Solo 2 device")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(App::new("aid").about("Prints the application's AID"))
                        .subcommand(
                            App::new("generate-ed255-key")
                                .about("Generates a new Trussed Ed255 attestation key"),
                        )
                        .subcommand(
                            App::new("generate-p256-key")
                                .about("Generates a new Trussed P256 attestation key"),
                        )
                        .subcommand(
                            App::new("generate-x255-key")
                                .about("Generates a new Trussed X255 agreement key"),
                        )
                        .subcommand(
                            App::new("store-ed255-cert")
                                .about("Stores a Trussed Ed255 attestation certificate")
                                .arg(
                                    Arg::new("DER")
                                        .help("Certificate in DER format")
                                        .required(true),
                                ),
                        )
                        .subcommand(
                            App::new("store-p256-cert")
                                .about("Stores a Trussed P256 attestation certificate")
                                .arg(
                                    Arg::new("DER")
                                        .help("Certificate in DER format")
                                        .required(true),
                                ),
                        )
                        .subcommand(
                            App::new("store-x255-cert")
                                .about("Stores a Trussed X255 attestation certificate")
                                .arg(
                                    Arg::new("DER")
                                        .help("Certificate in DER format")
                                        .required(true),
                                ),
                        )
                        .subcommand(
                            App::new("store-t1-pubkey")
                                .about("Stores the Trussed T1 intermediate public key")
                                .arg(
                                    Arg::new("BYTES")
                                        .help("Ed255 public key (raw, 32B)")
                                        .required(true),
                                ),
                        )
                        .subcommand(
                            App::new("store-fido-batch-key")
                                .about("Stores the FIDO batch attestation private key")
                                .arg(
                                    Arg::new("KEY")
                                        .help("P256 private key in internal format")
                                        .required(true),
                                ),
                        )
                        .subcommand(
                            App::new("store-fido-batch-cert")
                                .about("Stores the FIDO batch attestation certificate")
                                .arg(
                                    Arg::new("CERT")
                                        .help("Attestation certificate")
                                        .required(true),
                                ),
                        )
                        .subcommand(
                            App::new("reformat-filesystem")
                                .about("Reformats the internal filesystem"),
                        )
                        .subcommand(App::new("boot-to-bootrom").about("Boot to ROM bootloader"))
                        .subcommand(App::new("uuid").about("UUID (serial number)"))
                        .subcommand(
                            App::new("write-file")
                                .about("Writes binary file to specified path")
                                .arg(
                                    Arg::new("DATA")
                                        .help("binary data file")
                                        .required(true),
                                )
                                .arg(
                                    Arg::new("PATH")
                                        .help("path in internal filesystem")
                                        .required(true),
                                ),
                        ),
                )
                .subcommand(
                    App::new("piv")
                        .about("PIV (personal identity verification) app")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(App::new("aid").about("Prints the application's AID")),
                )
                .subcommand(
                    App::new("qa")
                        .about("app for factory QA of Solo 2 devices")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(App::new("aid").about("Prints the application's AID")),
                ),
        )
        // dev PKI
        .subcommand(
            App::new("pki")
                .about("PKI-related")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .setting(AppSettings::InferSubcommands)
                .subcommand(
                    App::new("ca")
                        .about("CA-related")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(
                            App::new("fetch-certificate")
                                .about("Fetch one of the well-known Solo 2 PKI certificates in DER format")
                                .arg(
                                    Arg::new("AUTHORITY")
                                        .help("name of authority, e.g. R1, T1, S3, etc.")
                                        .required(true),
                                )
                        )
                )
                .subcommand(
                    App::new("dev")
                        .about("PKI for development")
                        .setting(AppSettings::SubcommandRequiredElseHelp)
                        .setting(AppSettings::InferSubcommands)
                        .subcommand(
                            App::new("fido")
                                .about("Generates a self-signed FIDO batch attestation cert+key")
                                .arg(
                                    Arg::new("KEY")
                                        .help("output file, for private P256 key in binary format")
                                        .required(true),
                                )
                                .arg(
                                    Arg::new("CERT")
                                        .help("output file, for self-signed certificate in DER format")
                                        .required(true),
                                ),
                        ),
                    ),

        )
        // inherited from lpc55-host
        .subcommand(
            App::new("provision")
                // .version(crate_version!())
                // .long_version(LONG_VERSION.as_str())
                .about("Runs a sequence of bootloader commands defined in the config file.")
                .arg(
                    Arg::new("CONFIG")
                        .help("Configuration file containing settings")
                        .required(true),
                ),
        )
        .subcommand(
            App::new("bootloader")
                .about("Interact with bootloader")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .setting(AppSettings::InferSubcommands)
                .subcommand(
                    App::new("reboot")
                        .about("Reboots (into device if firmware is valid)"),
                )
                .subcommand(
                    App::new("list")
                    .visible_alias("ls")
                    .about("Lists all available bootloaders")
                ),
        )
        .subcommand(
            App::new("list")
                .visible_alias("ls")
                .about("List all device candidates.")
        )
        .subcommand(
            App::new("update")
                .about("Update to latest firmware published by SoloKeys. Warns on Major updates.")
                .arg(
                    Arg::new("yes")
                        .short('y')
                        .long("yes")
                        .help("DANGER! Proceed with major updates without prompt.")
                        .required(false),
                )
                .arg(
                    Arg::new("all")
                        .short('a')
                        .long("all")
                        .help("Update all connected SoloKeys Solo 2 devices.")
                        .required(false),
                )
                .arg(
                    Arg::new("FIRMWARE")
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
