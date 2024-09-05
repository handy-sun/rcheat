mod ctrl;
mod load_elf;

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
    about = "rcheat - Get/modify variable's value in another Linux/Unix running process.",
    long_about = None
)]
pub struct Args {
    #[arg(short = 'v', long = "version")]
    version: bool,
    /// Process id to trace
    #[arg(short = 'p', long = "pid", default_value_t = -1)]
    pid: pid_t,
    /// Keyword of the variable which want to search
    #[arg(short, long, default_value = "")]
    keyword: String,
}

fn run_main(arg: Args) -> AnyError {
    shadow!(build);

    if arg.version {
        println!("version     : {}", build::PKG_VERSION);
        println!("branch      : {} (clean: {})", build::BRANCH, build::GIT_CLEAN);
        println!("commit_hash : {}", build::SHORT_COMMIT);
        println!("build_time  : {}", build::BUILD_TIME);
        println!("build_env   : {}, {}", build::RUST_VERSION, build::RUST_CHANNEL);
        return Ok(());
    }

    // TODO: get_max_pid from /proc/sys/kernel/pid_max
    if arg.pid <= 1 {
        return Err(anyhow!("pid: {} is illegal!", arg.pid));
    }

    if arg.keyword.is_empty() {
        return Err(anyhow!("the input of keyword option is empty!"));
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
