use crate::Args;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::result::Result::Ok;
use std::{mem, time::Instant};

use chrono::Local;

use anyhow::{anyhow, Context, Error};

use nix::fcntl::readlink;
use nix::libc::{c_long, c_void, pid_t};
use nix::sys::ptrace;
use nix::sys::signal::Signal::SIGCONT;
use nix::sys::wait;
use nix::unistd::Pid;

use bytes::{Buf, BufMut, BytesMut};

fn pass_or_exit(ret: &nix::Result<()>, msg: &str) -> Result<(), Error> {
    match ret {
        Ok(_) => Ok(()),
        Err(errno) => {
            let os_err = std::io::Error::last_os_error();
            Err(anyhow!("{} failed: {:?}, {:?}", msg, errno, os_err))
        }
    }
}

fn get_abs_path(pid: pid_t) -> Result<String, Error> {
    let proc_exe = format!("/proc/{}/exe", pid);
    let path = Path::new(&proc_exe);
    match readlink(path) {
        Ok(os_link) => match os_link.into_string() {
            Ok(string) => Ok(string),
            Err(os_str) => Err(anyhow!("OsString:{:?} to String failed", os_str)),
        },
        Err(errno) => Err(anyhow!("readlink failed: {:?}", errno)),
    }
}

fn get_base_addr(pid: pid_t, exe_path: &String) -> Result<u64, Error> {
    let proc_maps = format!("/proc/{}/maps", pid);
    let path = Path::new(&proc_maps);

    let file = File::open(path).map_err(|err| anyhow!("Problem open file {:?}: {}", proc_maps, err))?;

    let file_reader = BufReader::new(file);
    for line_res in file_reader.lines() {
        let line = line_res.context("not")?;
        let cols: Vec<_> = line.split_whitespace().collect();
        if cols.len() < 6 {
            continue;
        }

        if cols.get(2) == Some(&"00000000") && cols.get(5) == Some(&exe_path.as_str()) {
            let col_0 = cols.get(0).ok_or_else(|| anyhow!("Column 0 is missing"))?;

            match col_0.find('-') {
                Some(pos) => {
                    let hex_str = &col_0[0..pos];
                    return u64::from_str_radix(hex_str, 16)
                        .map_err(|_| anyhow!("Failed to parse hex string: {}", hex_str));
                }
                None => return Err(anyhow!("Column 0 must contains '-'")),
            }
        }
    }

    Err(anyhow!("The maps file don't contain the base address"))
}

fn restore_process_to_run(pid: Pid, err: Error) -> Result<(), Error> {
    pass_or_exit(&ptrace::cont(pid, SIGCONT), "ptrace cont(SIGCONT")?;
    // pass_or_exit(&ptrace::detach(pid, SIGCONT), "ptrace detach")?; try it
    Err(err)
}

pub fn trace(arg: Args) -> Result<(), Error> {
    let pid = Pid::from_raw(arg.raw_pid);

    let start = Instant::now();
    let exe_path = get_abs_path(arg.raw_pid)?;

    println!("exe_path: {}", exe_path);
    println!("address: {}", arg.address);

    let dec = get_base_addr(arg.raw_pid, &exe_path)?;
    println!("get_base_addr: {:#x} ({})", dec, dec);
    pass_or_exit(&ptrace::attach(pid), "ptrace attach")?;

    match wait::waitpid(pid, None) {
        Ok(_status) => {
            println!("waitpid succeeded, status: {:?}", _status);
        }
        Err(e) => {
            return restore_process_to_run(pid, anyhow!("waitpid failed: {:?}", e));
        }
    }
    let val = arg.address.parse::<i64>().unwrap();
    let addr = ptrace::AddressType::from(val as *mut c_void);
    const COUNT: usize = 10; // TEST
    let mut peek_buf = BytesMut::with_capacity(COUNT * mem::size_of::<c_long>());

    for i in 0..COUNT {
        match ptrace::read(pid, addr.wrapping_add(i * mem::size_of::<c_long>())) {
            Ok(long_data) => {
                peek_buf.put_i64_le(long_data);
                // peek_buf.put::<c_long>(long_data); // TODO
            }
            Err(errno) => {
                return restore_process_to_run(pid, anyhow!("ptrace peekdata failed: {:?}", errno));
            }
        }
    }

    pass_or_exit(&ptrace::detach(pid, None), "ptrace detach")?;

    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);

    // println!("{:?}", peek_buf);
    let mut out_buf = peek_buf.clone();

    let per_line = 8;
    while out_buf.remaining() > 0 {
        if out_buf.remaining() < per_line {
            println!("{:?}", out_buf.get(0..out_buf.remaining()).unwrap());
            break;
        }
        println!("{:?}", out_buf.get(0..per_line).unwrap());
        out_buf.advance(per_line);
    }
    println!("{}", Local::now().format("%Y-%m-%d %H:%M:%S"));
    Ok(())
}
