mod ctrl;

use anyhow::{anyhow, Error};
use clap::Parser;
use nix::libc::pid_t;

use shadow_rs::shadow;

use crate::ctrl::trace;

#[derive(Clone, Debug, Parser)]
#[clap(name = "rcheat", about = "rcheat - Cheat a running linux process' memory")]
pub struct Args {
    #[clap(short = 'v', long = "version")]
    version: bool,
    #[clap(short = 'p', long = "pid", default_value = "0")]
    raw_pid: pid_t,
    #[clap(short = 'a', long = "address", default_value = "")]
    address: String,
}

fn run_main(arg: Args) -> Result<(), Error> {
    shadow!(build);

    if arg.version {
        println!("version:     {}", build::PKG_VERSION);
        println!("branch:      {} (git_clean: {})", build::BRANCH, build::GIT_CLEAN);
        println!("commit_hash: {}", build::SHORT_COMMIT);
        println!("build_time:  {}", build::BUILD_TIME);
        println!("build_env:   {}, {}", build::RUST_VERSION, build::RUST_CHANNEL);
        return Ok(());
    }

    if arg.raw_pid <= 1 {
        return Err(anyhow!("pid: {} is illegal!", arg.raw_pid));
    }

    if arg.address.is_empty() {
        return Err(anyhow!("address is empty!"));
    }

    trace(arg)
}

fn main() {
    let arg = Args::parse();
    match run_main(arg) {
        Ok(()) => (),
        Err(err) => {
            let io_kind = err
                .root_cause()
                .downcast_ref::<std::io::Error>()
                .map(std::io::Error::kind);

            if io_kind != Some(std::io::ErrorKind::BrokenPipe) {
                eprintln!("{}", err);
            }
            std::process::exit(1);
        }
    }
}
