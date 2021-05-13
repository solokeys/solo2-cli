use clap::{self, crate_authors, crate_version, App, Arg, SubCommand};

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
        .setting(clap::AppSettings::SubcommandRequiredElseHelp)
        // apps
        .subcommand(
            SubCommand::with_name("app")
                .about("app interactions")
                .setting(clap::AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("management")
                    .about("management app")
                    .setting(clap::AppSettings::SubcommandRequiredElseHelp)
                    .subcommand(
                        SubCommand::with_name("reboot")
                            .about("reboot device to regular mode")
                    )
                    .subcommand(
                        SubCommand::with_name("boot-to-bootrom")
                            .about("reboot device to bootloader mode")
                    )
                    .subcommand(
                        SubCommand::with_name("uuid")
                            .about("UUID (serial number)")
                    )
                    .subcommand(
                        SubCommand::with_name("version")
                            .about("version")
                    )
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
                .setting(clap::AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("reboot")
                        .about("reboot (into device if firmware is valid)"),
                )
        )
        // .subcommand(
        //     SubCommand::with_name("reboot")
        //         .version(crate_version!())
        //         .long_version(LONG_VERSION.as_str())
        //         .about("reboot device"),
        // )

    ;

    cli
}

/// Return the "long" format of lpc55's version string.
///
/// If a revision hash is given, then it is used. If one isn't given, then
/// the LPC55_BUILD_GIT_HASH env var is inspected for it. If that isn't set,
/// then a revision hash is not included in the version string returned.
pub fn long_version(revision_hash: Option<&str>) -> String {
    // Do we have a git hash?
    // (Yes, if ripgrep was built on a machine with `git` installed.)
    let hash = match revision_hash.or(option_env!("LPC55_BUILD_GIT_HASH")) {
        None => String::new(),
        Some(githash) => format!(" (rev {})", githash),
    };
    format!("{}{}", crate_version!(), hash)
}
