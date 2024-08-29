mod ctrl;

use anyhow::{anyhow, Error};
use clap::Parser;

use nix::libc::pid_t;

use shadow_rs::shadow;

use crate::ctrl::trace;

type AnyError = Result<(), Error>;

#[derive(Clone, Debug, Parser)]
#[command(
    disable_version_flag = true,
    name = "rcheat",
    about = "rcheat - Cheat a running linux process' memory.",
    long_about = None
)]
pub struct Args {
    #[arg(short = 'v', long = "version")]
    version: bool,
    /// Process id to trace
    #[arg(short = 'p', long = "pid", default_value_t = -1)]
    pid: pid_t,
    /// Address of global var
    #[arg(short = 'a', long = "address", default_value = "")]
    address: String,
}

fn run_main(arg: Args) -> AnyError {
    shadow!(build);

    if arg.version {
        println!("version     : {}", build::PKG_VERSION);
        println!("branch      : {} (git_clean: {})", build::BRANCH, build::GIT_CLEAN);
        println!("commit_hash : {}", build::SHORT_COMMIT);
        println!("build_time  : {}", build::BUILD_TIME);
        println!("build_env   : {}, {}", build::RUST_VERSION, build::RUST_CHANNEL);
        return Ok(());
    }

    // TODO: get_max_pid from /proc/sys/kernel/pid_max
    if arg.pid <= 1 {
        return Err(anyhow!("pid: {} is illegal!", arg.pid));
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
