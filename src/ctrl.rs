use crate::AnyError;
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

fn pass_or_exit(ret: &nix::Result<()>, msg: &str) -> AnyError {
    match ret {
        Ok(_) => Ok(()),
        Err(errno) => {
            let os_err = std::io::Error::last_os_error();
            Err(anyhow!("{} failed: {:?}, {:?}", msg, errno, os_err))
        }
    }
}

fn get_abs_path(tracked_pid: pid_t) -> Result<String, Error> {
    let proc_exe = format!("/proc/{}/exe", tracked_pid);
    let path = Path::new(&proc_exe);
    match readlink(path) {
        Ok(os_link) => match os_link.into_string() {
            Ok(string) => Ok(string),
            Err(os_str) => Err(anyhow!("OsString:{:?} to String failed", os_str)),
        },
        Err(errno) => Err(anyhow!("readlink failed: {:?}", errno)),
    }
}

fn get_base_addr<R: BufRead>(buf_read: R, exe_path: &str) -> Result<u64, Error> {
    for line_res in buf_read.lines() {
        let line = line_res.context("not")?;
        let cols: Vec<_> = line.split_whitespace().collect();
        if cols.len() < 6 {
            continue;
        }

        if cols.get(2) == Some(&"00000000") && cols.get(5) == Some(&exe_path) {
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

fn restore_process_to_run(tracked_pid: Pid, err: Error) -> AnyError {
    pass_or_exit(&ptrace::cont(tracked_pid, SIGCONT), "ptrace_cont(SIGCONT)")?;
    // pass_or_exit(&ptrace::detach(tracked_pid, SIGCONT), "ptrace_detach(SIGCONT)")?;
    Err(err)
}

fn parse_various_input(indef: &String) -> Result<u64, Error> {
    let dec;
    if indef.to_lowercase().starts_with("0x") {
        let no_pre = &indef[2..];
        dec = u64::from_str_radix(no_pre, 16)
            .map_err(|_| anyhow!("Failed to parse hex string: {}", indef))?;
    } else {
        dec = indef
            .parse::<u64>()
            .map_err(|err| anyhow!("Parse to u64 failed: {:?}", err))?;
    }
    Ok(dec)
}

pub fn trace(arg: Args) -> AnyError {
    let addr_dec = parse_various_input(&arg.address)?;
    let tracked_pid = Pid::from_raw(arg.pid);

    let start = Instant::now();
    let exe_path = get_abs_path(arg.pid)?;

    println!("exe_path: {}", exe_path);
    println!("address: {:#x} ({})", addr_dec, addr_dec);

    let proc_maps = format!("/proc/{}/maps", tracked_pid);
    let file = File::open(Path::new(&proc_maps))
        .map_err(|err| anyhow!("Problem open file {:?}: {}", proc_maps, err))?;

    let file_reader = BufReader::new(file);
    let addr_val = get_base_addr(file_reader, &exe_path.as_str())?;
    println!("base_addr: {:#x} ({})", addr_val, addr_val);

    pass_or_exit(&ptrace::attach(tracked_pid), "ptrace_attach")?;

    match wait::waitpid(tracked_pid, None) {
        Ok(_status) => {
            println!("waitpid succeeded, status: {:?}", _status);
        }
        Err(e) => {
            return restore_process_to_run(tracked_pid, anyhow!("waitpid failed: {:?}", e));
        }
    }

    let addr = ptrace::AddressType::from(addr_dec as *mut c_void);
    const COUNT: usize = 10; // TEST
    let mut peek_buf = BytesMut::with_capacity(COUNT * mem::size_of::<c_long>());

    for i in 0..COUNT {
        match ptrace::read(tracked_pid, addr.wrapping_add(i * mem::size_of::<c_long>())) {
            Ok(long_data) => {
                peek_buf.put_i64_le(long_data);
                // peek_buf.put::<c_long>(long_data); // TODO?
            }
            Err(errno) => {
                return restore_process_to_run(tracked_pid, anyhow!("peekdata at {:?}: {:?}", addr, errno));
            }
        }
    }

    pass_or_exit(&ptrace::detach(tracked_pid, None), "ptrace_detach")?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn _get_base_addr() {
        let exe_abs_path = "/usr/bin/test1";
        let contents = "
00400000-004ac000 r-xp 00000000 08:02 8918620   /usr/bin/test1
006ab000-006ac000 r--p 000ab000 08:02 8918620   /usr/bin/test1
006ac000-006b2000 rw-p 000ac000 08:02 8918620   /usr/bin/test1
0092f000-00a9b000 rw-p 00000000 00:00 0         [heap]";
        let buf_rdr = BufReader::new(contents.as_bytes());
        assert_eq!(get_base_addr(buf_rdr, &exe_abs_path).unwrap_or_default(), 0x400000);

        let exe_abs_path = "/usr/bin/test2";
        let contents = "
7fc5f7864000-7fc5f7874000 r-xp 00000000 08:02 8918670 /usr/lib64/libtest
7fc5f7874000-7fc5f7a73000 ---p 00010000 08:02 8918670 /usr/lib64/libtest";
        let buf_rdr = BufReader::new(contents.as_bytes());
        assert_eq!(get_base_addr(buf_rdr, &exe_abs_path).unwrap_or_default(), 0);
    }
}
