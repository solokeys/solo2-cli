use std::env;
use std::fs;
use std::path;
use std::process;

#[cfg(feature = "cli")]
#[path = "src/bin/solo2/cli.rs"]
mod cli;

fn main() {
    // OUT_DIR is set by Cargo and it's where any additional build artifacts
    // are written.
    let env_outdir = match env::var_os("OUT_DIR") {
        Some(outdir) => outdir,
        None => {
            eprintln!(
                "OUT_DIR environment variable not defined. \
                 Please file a bug: \
                 https://github.com/BurntSushi/ripgrep/issues/new"
            );
            process::exit(1);
        }
    };

    // // empty file that is used in scripts/cargo-out-dir
    // let stamp_path = path::Path::new(&env_outdir).join("solo2-stamp");
    // if let Err(err) = fs::File::create(&stamp_path) {
    //     panic!("failed to write {}: {}", stamp_path.display(), err);
    // }

    // place side by side with binaries
    let outdir = path::PathBuf::from(path::PathBuf::from(env_outdir).ancestors().nth(3).unwrap());
    fs::create_dir_all(&outdir).unwrap();
    println!("{:?}", &outdir);

    #[cfg(feature = "cli")]
    {
        use clap::Shell;

        // Use clap to build completion files.
        // Pro-tip: use `fd -HIe bash` to get OUT_DIR
        let mut app = cli::cli();
        app.gen_completions("solo2", Shell::Bash, &outdir);
        app.gen_completions("solo2", Shell::Fish, &outdir);
        app.gen_completions("solo2", Shell::PowerShell, &outdir);
        // // Note that we do not use clap's support for zsh. Instead, zsh completions
        // // are manually maintained in `complete/_rg`.
        app.gen_completions("solo2", Shell::Zsh, &outdir);
    }

    // // Make the current git hash available to the build.
    // if let Some(rev) = git_revision_hash() {
    //     // this works, but it doesn't get picked up in app :/
    //     println!("cargo:rustc-env=LPC55_BUILD_GIT_HASH={}", rev);
    // }
}

// fn git_revision_hash() -> Option<String> {
//     let result = process::Command::new("git")
//         .args(&["rev-parse", "--short=10", "HEAD"])
//         .output();
//     result.ok().and_then(|output| {
//         let v = String::from_utf8_lossy(&output.stdout).trim().to_string();
//         if v.is_empty() {
//             None
//         } else {
//             Some(v)
//         }
//     })
// }
