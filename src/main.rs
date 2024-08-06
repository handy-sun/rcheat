use std::{env, process};

use nix::sys::ptrace;
use nix::sys::wait;
use nix::unistd::Pid;
use nix::libc::pid_t;

fn print_or_exit(ret: &nix::Result<()>, msg: &str) {
    match ret {
        Ok(_) => println!("{} succeeded", msg),
        Err(e) => {
            let os_err = std::io::Error::last_os_error();
            eprintln!("{} failed: {:?}, {:?}", msg, e, os_err);
            process::exit(os_err.raw_os_error().unwrap_or(-1));
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arg_vec: Vec<String> = env::args().collect();
    if arg_vec.len() < 2 {
        eprintln!("please input pid!");
        process::exit(1);
    }

    let raw_pid: pid_t = arg_vec.get(1).unwrap_or(&String::from("0")).parse()?;
    if raw_pid < 2 {
        eprintln!("the pid must greater than 1!");
        process::exit(1);
    }
    let pid = Pid::from_raw(raw_pid);

    println!("self pid is: {:?}", nix::unistd::getpid());

    print_or_exit(&ptrace::attach(pid), "ptrace attach");

    match wait::waitpid(pid, None) {
        Ok(status) => println!("waitpid succeeded, status: {:?}", status),
        Err(e) => {
            let os_err = std::io::Error::last_os_error();
            eprintln!("waitpid failed: {:?}, {:?}", e, os_err);
            process::exit(os_err.raw_os_error().unwrap_or(-1));
        }
    }

    print_or_exit(&ptrace::detach(pid, None), "ptrace detach");

    Ok(())
}
