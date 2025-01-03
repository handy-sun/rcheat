mod ctrl;
mod elf;
mod fmt_dump;
// #[macro_use]
mod lua;
mod macros;
mod qpid;

use clap::Parser;
use nix::libc::pid_t;
use owo_colors::OwoColorize;
// use shadow_rs::shadow;

use ctrl::further_parse;

type AnyError = Result<(), anyhow::Error>;

#[derive(Clone, Debug, Parser)]
#[command(
    disable_version_flag = true,
    name = "rcheat",
    about = "rcheat - Get/modify variable's value in another Linux running process.",
    long_about = None
)]
pub struct Args {
    #[arg(short = 'v', long = "version")]
    version: bool,
    /// Process id to trace
    #[arg(short, long)]
    pid: Option<pid_t>,
    /// Name(or part of name) of the process
    #[arg(short, long)]
    name: Option<String>,
    /// Keyword(or regex expression) of the variable which want to search
    #[arg(short, long)]
    keyword: Option<String>,
    /// Format output 'hex' or 'dec', 'lua'
    #[arg(short, long)]
    format: Option<String>,
}

fn run_main(arg: Args) -> AnyError {
    // shadow!(build);
    if arg.version {
        // let short_commit = ;
        let build_time = option_env!("RCHEAT_BUILD_TIME").unwrap_or("");
        let is_clean_commit = option_env!("RCHEAT_GIT_IS_CLEAN_COMMIT").is_some();
        let version = option_env!("RCHEAT_GIT_TAG_VERSION").unwrap_or("0.0.0");

        // If there are any pending changes, show the hash in red
        let hash_with_color = if is_clean_commit {
            option_env!("RCHEAT_BUILD_GIT_HASH").unwrap_or("").to_string()
        } else {
            option_env!("RCHEAT_BUILD_GIT_HASH")
                .unwrap_or("")
                .red()
                .to_string()
        };
        println!("rcheat {} ({} {})", version, hash_with_color, build_time);
        return Ok(());
    }

    further_parse(arg)
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
