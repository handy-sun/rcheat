// extern crate inotify;
// extern crate libc;

// use std::{env, os::unix::raw::pid_t};
use std::env;
// use std::io;
// use std::mem;
use std::process;
use libc::{
    // c_long, 
    // c_void, 
    // c_uint, 
    pid_t
};
 
// fn ptrace(request: c_uint, pid: pid_t, addr: *mut c_void, data: *mut c_void) -> c_long {
//     unsafe {
//         libc::ptrace(request, pid, addr, data)
//     }
// }

// fn type_of<T> (_: T) -> &'static str {
//     std::any::type_name::<T >()
// }

fn main()
// -> io::Result<()>
{
    let sys_args = env::args();
    let mut pid: pid_t = 0;
    for (i, arg) in sys_args.enumerate() {
        if i == 1 {
            pid = arg.parse().unwrap();
        }
    }

    if let 0 = pid {
        eprintln!("pid is zero");
        process::exit(1);
    }

    // let result = ptrace(libc::PTRACE_TRACEME, 0, std::ptr::null_mut(), std::ptr::null_mut());
    let result = unsafe {
        libc::ptrace(libc::PTRACE_ATTACH, pid, 0, 0)
    };
    match result <= 0 {
        true => println!("res={}, {:?}", result, std::io::Error::last_os_error().into_inner()),
        false => println!("ptrace succeeded"),
        // Ok(Self { process }),
    }

    // let result = ptrace(libc::PTRACE_TRACEME, 0, std::ptr::null_mut(), std::ptr::null_mut());

    // let result = ptrace::attach(pid).ok().expect("Could not attach to child");
    // if result == 0 {
    //     println!("ptrace succeeded");
    // } else {
    //     println!("ptrace failed: {}", result);
    // }
}
