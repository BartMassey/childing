use std::io::prelude::*;
use std::os::unix::process::CommandExt;
use std::process::Command;

use libc::{c_int, fork, waitpid};
use os_pipe::pipe;
use perfcnt::{*, linux::*};

fn main() {
    // Make a pipe for unblocking the child later.
    let (reader, mut writer) = pipe().unwrap();
    let mut child_reader = reader.try_clone().unwrap();
    // Fork.
    eprintln!("spawning");
    let pid = match unsafe { fork() } {
        0 => {
            eprintln!("child started");
            let mut buf = [0];
            let nread = child_reader.read(&mut buf).unwrap();
            assert_eq!(nread, 1);
            assert_eq!(buf[0], 1);
            eprintln!("child command starting");
            let mut command = Command::new("wc");
            command.arg("/usr/share/dict/words");
            let e = command.exec();
            panic!("child command failed: {}", e);
        }
        pid => pid,
    };
    eprintln!("child {}", pid);
    // XXX You'd want to make a perf counter for the child pid here.
    // I have no idea how to do this.
    let mut pc: PerfCounter =
        PerfCounterBuilderLinux::from_hardware_event(HardwareEventType::CacheMisses)
        .finish()
        .unwrap();
    // Start the child perf counter now.
    pc.start().unwrap();
    eprintln!("pc started");
    // Unblock the child.
    writer.write(&[1]).unwrap();
    drop(writer);
    // Wait for the parent.
    eprintln!("waiting for parent");
    let mut status: c_int = 0;
    let result = unsafe { waitpid(
        pid,
        (&mut status) as *mut c_int,
        0,
    )};
    assert_eq!(result, pid);
    assert_eq!(status, 0);
    // Stop the perf counter and return the answer.
    pc.stop().unwrap();
    let count = pc.read().unwrap();
    println!("{}", count);
}
