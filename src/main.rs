use std::{env, mem, process, ptr, time::Instant};

use chrono::Local;

use nix::libc::{c_long, c_void, pid_t};
use nix::sys::ptrace;
use nix::sys::wait;
use nix::unistd::Pid;

// temp value for test
const A_SIZE: usize = 9;

#[repr(packed)]
#[derive(Debug)]
struct PackTot {
    key: u32,
    ba: [u8; 5],
}

const NULL_PTT: PackTot = PackTot { key: 0, ba: [0u8; 5] };

fn pass_or_exit(ret: &nix::Result<()>, msg: &str) {
    match ret {
        Ok(_) => {}
        Err(e) => {
            let os_err = std::io::Error::last_os_error();
            eprintln!("{} failed: {:?}, {:?}", msg, e, os_err);
            process::exit(os_err.raw_os_error().unwrap_or(-1));
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arg_vec: Vec<String> = env::args().collect();
    if arg_vec.len() < 3 {
        eprintln!("please input pid!");
        process::exit(1);
    }

    let raw_pid: pid_t = arg_vec.get(1).unwrap_or(&String::from("0")).parse()?;
    if raw_pid < 2 {
        eprintln!("the pid must greater than 1!");
        process::exit(1);
    }

    let raw_addr: u64 = arg_vec.get(2).unwrap_or(&String::from("0")).parse()?;
    if raw_addr < 1 {
        eprintln!("the addr must greater than 1!");
        process::exit(1);
    }

    let pid = Pid::from_raw(raw_pid);

    println!("traced pid: {:?}, self pid: {:?}", pid, nix::unistd::getpid());
    // let now: DateTime<Local> = Local::now();
    let start = Instant::now();

    pass_or_exit(&ptrace::attach(pid), "ptrace attach");

    match wait::waitpid(pid, None) {
        Ok(_status) => {
            // println!("waitpid succeeded, status: {:?}", status);
        }
        Err(e) => {
            let os_err = std::io::Error::last_os_error();
            eprintln!("waitpid failed: {:?}, {:?}", e, os_err);
            process::exit(os_err.raw_os_error().unwrap_or(-1));
        }
    }

    let addr = ptrace::AddressType::from(raw_addr as *mut c_void);
    let mut peek_arr: [c_long; A_SIZE] = [0; A_SIZE];

    for i in 0..peek_arr.len() {
        match ptrace::read(pid, addr.wrapping_add(i * mem::size_of::<c_long>())) {
            Ok(long_data) => {
                peek_arr[i] = long_data;
            }
            Err(e) => {
                let os_err = std::io::Error::last_os_error();
                eprintln!("ptrace peekdata failed: {:?}", e);
                process::exit(os_err.raw_os_error().unwrap_or(-1));
            }
        }
    }

    pass_or_exit(&ptrace::detach(pid, None), "ptrace detach");

    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);

    // println!("{}", mem::size_of::<PackTot>());
    // println!(" {:#?}", peek_arr);

    let mut off = 0;
    while off + mem::size_of::<PackTot>() <= mem::size_of_val(&peek_arr) {
        let ptt = NULL_PTT;
        unsafe {
            let src = (peek_arr.as_ptr() as *const u8).add(off);
            let dst: *mut u8 = mem::transmute(&ptt);
            ptr::copy_nonoverlapping(src, dst, mem::size_of::<PackTot>());
        }
        println!("{} {:?}", ptt.key as u32, ptt.ba);
        off += mem::size_of::<PackTot>();
    }

    println!("{}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    // println!("[{:02}:{:02}:{:02}.{:06}]", date_time.hour(), date_time.minute(), date_time.second(), date_time.nanosecond() / 1000);

    Ok(())
}
