use crate::ceil_to_multiple;
use crate::elf::ElfMgr;
use crate::fmt_dump::*;
use crate::AnyError;
use crate::Args;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::{mem, time::Instant};

use anyhow::{anyhow, Context, Error};

use nix::fcntl::readlink;
use nix::libc::{c_long, pid_t};
use nix::sys::ptrace;
use nix::sys::signal::Signal::SIGCONT;
use nix::sys::wait;
use nix::unistd::Pid;

use bytes::{BufMut, BytesMut};

const LONG_SIZE: usize = mem::size_of::<c_long>();

/// `maps column means`

/// vm_addr range `addr_begin`-`addr_end`
const ADDR_RANGE: usize = 0;

/// permission of this area `rwx(p/s)`
// const PERMISSION: usize = 1;

/// offset from the base addr
const OFFSET: usize = 2;

/// main device id : secondary device id
// const MAIN_2ND_DEV: usize = 3;

/// inode of the file
// const INODE: usize = 4;

/// absolute path of ref file
const FILE_ABS_PATH: usize = 5;

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

        if cols.get(OFFSET) == Some(&"00000000") && cols.get(FILE_ABS_PATH) == Some(&exe_path) {
            let col_0 = cols
                .get(ADDR_RANGE)
                .ok_or_else(|| anyhow!("Column ADDR_RANGE(0) is missing"))?;
            match col_0.find('-') {
                Some(pos) => {
                    let hex_str = &col_0[..pos];
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

    let start = Instant::now();
    // let elf_data = match_sym_entry(&elf_bytes, &arg.keyword)?;
    let elf_mgr = ElfMgr::prase_from(&elf_bytes)?;
    println!("[{:?}] Time of `parse elf`", start.elapsed());

    let entry = elf_mgr.select_sym_entry(&arg.keyword)?;

    let entry_addr = if elf_mgr.is_exec_elf() {
        entry.obj_addr()
    } else if elf_mgr.is_dyn_elf() {
        let proc_maps = format!("/proc/{}/maps", tracked_pid);
        let file = File::open(Path::new(&proc_maps))
            .map_err(|err| anyhow!("Problem open file {:?}: {}", proc_maps, err))?;
        let file_reader = BufReader::new(file);
        let base_addr = get_base_addr(file_reader, exe_path.as_str())?;
        println!("base_addr: {:#x} ({})", base_addr, base_addr);
        if let Some(total_addr) = base_addr.checked_add(entry.obj_addr()) {
            total_addr
        } else {
            return Err(anyhow!("Operation of base_addr add obj_addr exceeds the limit"));
        }
    } else {
        return Err(anyhow!("Unsupport e_type:"));
    };

    println!("entry address: {:#x}, size: {}", entry_addr, entry.obj_size());

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
    let var_sz = entry.obj_size() as usize;
    // It can be confirmed that this number(var_sz) must be greater than 0
    let mut peek_buf = BytesMut::with_capacity(ceil_to_multiple!(var_sz, LONG_SIZE));

    // The target address size less than c_long, only need read once, and then
    // truncate BytesMut to real size
    if var_sz < LONG_SIZE {
        match ptrace::read(tracked_pid, addr) {
            Ok(long_data) => {
                peek_buf.put(long_data.to_ne_bytes().as_ref());
                peek_buf.truncate(var_sz);
            }
            Err(errno) => {
                return restore_process_to_run(tracked_pid, anyhow!("peekdata at {:?}: {:?}", addr, errno));
            }
        };
    } else {
        let mut pos: usize = 0;
        while pos < var_sz {
            if pos + LONG_SIZE > var_sz {
                pos = var_sz - LONG_SIZE;
                peek_buf.truncate(pos);
            }
            match ptrace::read(tracked_pid, addr.wrapping_add(pos)) {
                Ok(long_data) => {
                    peek_buf.put(long_data.to_ne_bytes().as_ref());
                }
                Err(errno) => {
                    return restore_process_to_run(
                        tracked_pid,
                        anyhow!("peekdata at {:?}: {:?}", addr, errno),
                    );
                }
            }
            pos += LONG_SIZE;
        }
    }

    pass_or_exit(&ptrace::detach(tracked_pid, None), "ptrace_detach")?;

    println!("[{:?}] Time of `trace and peek`", start.elapsed());
    if let Some(bytes_ref) = peek_buf.get(..) {
        let out_content = if arg.format == "dec" {
            dump_to_dec_content(bytes_ref)
        } else {
            dump_to_hex_content(bytes_ref)
        };
        println!("\n{}", out_content);
        Ok(())
    } else {
        Err(anyhow!("Peek buf is empty"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn func_get_base_addr() {
        let exe_abs_path = "/usr/bin/test1";
        let contents = "
00400000-004ac000 r-xp 00000000 08:02 8918620   /usr/bin/test1
006ab000-006ac000 r--p 000ab000 08:02 8918620   /usr/bin/test1
006ac000-006b2000 rw-p 000ac000 08:02 8918620   /usr/bin/test1
0092f000-00a9b000 rw-p 00000000 00:00 0         [heap]";
        let buf_rdr = BufReader::new(contents.as_bytes());
        assert_eq!(get_base_addr(buf_rdr, exe_abs_path).unwrap_or_default(), 0x400000);

        let exe_abs_path = "/usr/bin/test2";
        let contents = "
7fc5f7864000-7fc5f7874000 r-xp 00000000 08:02 8918670 /usr/lib64/libtest
7fc5f7874000-7fc5f7a73000 ---p 00010000 08:02 8918670 /usr/lib64/libtest";
        let buf_rdr = BufReader::new(contents.as_bytes());
        assert_eq!(get_base_addr(buf_rdr, exe_abs_path).unwrap_or_default(), 0);
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
