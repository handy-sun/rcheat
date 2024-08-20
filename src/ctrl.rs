use crate::Args;

use std::{mem, time::Instant};
use std::path::Path;
use std::result::Result::Ok;
use std::ffi::OsString;

use chrono::Local;

use nix::libc::{c_long, c_void, pid_t};
use nix::sys::ptrace;
use nix::sys::wait;
use nix::unistd::Pid;
use nix::fcntl::readlink;
use anyhow::{anyhow, Error};

use bytes::{Buf, BufMut, BytesMut};


fn pass_or_exit(ret: &nix::Result<()>, msg: &str) -> Result<(), Error> {
    match ret {
        Ok(_) => Ok(()),
        Err(e) => {
            let os_err = std::io::Error::last_os_error();
            Err(anyhow!("{} failed: {:?}, {:?}", msg, e, os_err))
        }
    }
}

fn get_abs_path(pid: pid_t) -> Result<OsString, Error> {
    let proc_exe = format!("/proc/{}/exe", pid);
    let path = Path::new(&proc_exe);
    match readlink(path) {
        Ok(link) => Ok(link),
        Err(e) => Err(anyhow!("readlink failed: {:?}", e))
    }
}

// TODO
// fn get_base_addr(pid: pid_t) -> Result<u64, Error> {
//     let proc_maps = format!("/proc/{}/maps", pid);
//     let _bytes = std::fs::read(&proc_maps)
//         .map_err(|err|anyhow!("Problem reading file {:?}: {}", proc_maps, err))?;
//     return Ok(0);
// }

pub fn trace(arg: Args) -> Result<(), Error> {
    let pid = Pid::from_raw(arg.raw_pid);

    let start = Instant::now();
    println!("{:?}", get_abs_path(arg.raw_pid));
    pass_or_exit(&ptrace::attach(pid), "ptrace attach")?;

    match wait::waitpid(pid, None) {
        Ok(_status) => {
            println!("waitpid succeeded, status: {:?}", _status);
        }
        Err(e) => {
            return Err(anyhow!("waitpid failed: {:?}", e));
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
            },
            Err(e) => {
                return Err(anyhow!("ptrace peekdata failed: {:?}", e));
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
