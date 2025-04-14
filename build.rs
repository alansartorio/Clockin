use clap::CommandFactory;
use clap_complete::Shell;
use clap_complete::generate_to;
use std::env;
use std::io::Error;

include!("src/cli.rs");

fn main() -> Result<(), Error> {
    if let Ok(outdir) = env::var("OUT_DIR") {
        let mut cmd = Args::command();
        for shell in [
            Shell::Bash,
            //Shell::Elvish,
            Shell::Fish,
            //Shell::PowerShell,
            Shell::Zsh,
        ] {
            let path = generate_to(shell, &mut cmd, "clockin", &outdir)?;
            println!("cargo:warning=completion file is generated: {path:?}");
        }
    }


    Ok(())
}
