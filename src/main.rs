use libc::pid_t;
use std::env;
use std::process;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sys_args = env::args();
    let mut pid: pid_t = 0;
    for (i, arg) in sys_args.enumerate() {
        if i == 1 {
            pid = arg.parse().unwrap();
        }
    }

    println!("My pid is: {}", unsafe { libc::getpid() });

    if let 0 = pid {
        eprintln!("Target pid is zero!");
        process::exit(1);
    }

    let result = unsafe { libc::ptrace(libc::PTRACE_SEIZE, pid, 0, 0) };
    match result >= 0 {
        true => {
            println!("ptrace seize succeeded");
            return Ok(());
        }
        false => {
            let os_err = std::io::Error::last_os_error();
            eprintln!("result={}, {:?}", result, os_err);
            process::exit(os_err.raw_os_error().unwrap_or(-1));
        }
    }
}
