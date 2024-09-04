use crate::load_elf::match_sym_entry;
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
use nix::libc::{c_long, pid_t};
use nix::sys::ptrace;
use nix::sys::signal::Signal::SIGCONT;
use nix::sys::wait;
use nix::unistd::Pid;

use goblin::elf::header;

use bytes::{Buf, BufMut, BytesMut};

const LONG_SIZE: usize = mem::size_of::<c_long>();

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

pub fn trace(arg: Args) -> AnyError {
    let tracked_pid = Pid::from_raw(arg.pid);
    let exe_path = get_abs_path(arg.pid)?;
    println!("exe_path: {}", &exe_path);

    let elf_bytes =
        std::fs::read(&exe_path).map_err(|err| anyhow!("Problem reading file {:?}: {}", &exe_path, err))?;

    let entry_addr;
    let elf_data = match_sym_entry(&elf_bytes, &arg.keyword)?;
    let entry_size = elf_data.1;
    match elf_data.2 {
        // Some old linux distribution use this
        header::ET_EXEC => {
            entry_addr = elf_data.0;
        }
        // Shared object file use (ASLR)
        header::ET_DYN => {
            let proc_maps = format!("/proc/{}/maps", tracked_pid);
            let file = File::open(Path::new(&proc_maps))
                .map_err(|err| anyhow!("Problem open file {:?}: {}", proc_maps, err))?;
            let file_reader = BufReader::new(file);
            let base_addr = get_base_addr(file_reader, &exe_path.as_str())?;
            println!("base_addr: {:#x} ({})", base_addr, base_addr);
            entry_addr = base_addr + elf_data.0;
        }
        _ => return Err(anyhow!("Unsupport e_type: {}", elf_data.2)),
    }

    println!("entry address: {:#x}, size: {}", entry_addr, entry_size);

    let start = Instant::now();
    pass_or_exit(&ptrace::attach(tracked_pid), "ptrace_attach")?;

    match wait::waitpid(tracked_pid, None) {
        Ok(_status) => {
            println!("waitpid succeeded, status: {:?}", _status);
        }
        Err(e) => {
            return restore_process_to_run(tracked_pid, anyhow!("waitpid failed: {:?}", e));
        }
    }

    let addr = ptrace::AddressType::from(entry_addr as ptrace::AddressType);
    let var_sz = entry_size as usize;
    let mut peek_buf = BytesMut::with_capacity(var_sz);

    let mut pos: usize = 0;
    while pos < var_sz {
        if pos + LONG_SIZE > var_sz {
            print!("before pos: {}, ", pos);
            pos = var_sz - LONG_SIZE;
            peek_buf.truncate(pos);
            println!("now pos: {}", pos);
        }
        match ptrace::read(tracked_pid, addr.wrapping_add(pos)) {
            Ok(long_data) => {
                peek_buf.put(long_data.to_ne_bytes().as_ref());
            }
            Err(errno) => {
                return restore_process_to_run(tracked_pid, anyhow!("peekdata at {:?}: {:?}", addr, errno));
            }
        }
        pos += LONG_SIZE;
    }

    pass_or_exit(&ptrace::detach(tracked_pid, None), "ptrace_detach")?;

    let duration = start.elapsed();
    println!("Time elapsed: {:?}", duration);

    // for test temp
    let per_line = 16;
    let parts_len = 8;
    let mut out_buf = peek_buf.clone();
    while out_buf.remaining() > 0 {
        if out_buf.remaining() < per_line {
            print!("{:>3?}  ", out_buf.get(0..parts_len).unwrap());
            println!("{:>3?}", out_buf.get(parts_len..out_buf.remaining()).unwrap());
            break;
        }
        print!("{:>3?}  ", out_buf.get(0..parts_len).unwrap());
        println!("{:>3?}", out_buf.get(parts_len..per_line).unwrap());
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
        assert_eq!(
            get_base_addr(buf_rdr, &exe_abs_path).unwrap_or_default(),
            0x400000
        );

        let exe_abs_path = "/usr/bin/test2";
        let contents = "
7fc5f7864000-7fc5f7874000 r-xp 00000000 08:02 8918670 /usr/lib64/libtest
7fc5f7874000-7fc5f7a73000 ---p 00010000 08:02 8918670 /usr/lib64/libtest";
        let buf_rdr = BufReader::new(contents.as_bytes());
        assert_eq!(get_base_addr(buf_rdr, &exe_abs_path).unwrap_or_default(), 0);
    }

    #[test]
    fn check_c_long_write_and_read() {
        #[cfg(target_pointer_width = "64")]
        {
            let long_val: c_long = 0x12345678_90abcdef;
            let mut buf = BytesMut::new();
            buf.put(long_val.to_ne_bytes().as_ref());

            #[cfg(target_endian = "little")]
            assert_eq!(buf.get(..), Some(b"\xef\xcd\xab\x90\x78\x56\x34\x12".as_ref()));

            #[cfg(target_endian = "big")]
            assert_eq!(buf.get(..), Some(b"\x12\x34\x56\x78\x90\xab\xcd\xef".as_ref()));
        }

        #[cfg(target_pointer_width = "32")]
        {
            let long_val: c_long = 0x5678_cdef;
            let mut buf = BytesMut::new();
            buf.put(long_val.to_ne_bytes().as_ref());

            #[cfg(target_endian = "little")]
            assert_eq!(buf.get(..), Some(&[0xef, 0xcd, 0x78, 0x56][..]));

            #[cfg(target_endian = "big")]
            assert_eq!(buf.get(..), Some([0x56, 0x78, 0xcd, 0xef].as_ref()));
        }
    }
}
