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
use shadow_rs::shadow;

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
    shadow!(build);

    if arg.version {
        let commit_hash_with_clean_color: &str = if build::GIT_CLEAN {
            build::SHORT_COMMIT
        } else {
            &build::SHORT_COMMIT.red().to_string()
        };
        println!(
            "{} {} ({} {})",
            build::PROJECT_NAME,
            build::PKG_VERSION,
            commit_hash_with_clean_color,
            build::BUILD_TIME
        );
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
