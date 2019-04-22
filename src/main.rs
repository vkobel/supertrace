use nix::sys::{ptrace, signal, wait};
use nix::sys::wait::WaitStatus;
use std::process::Command;
use core::ffi::c_void;

mod x64_openflags;
mod alterable_command;
use alterable_command::alterable_command::AlterableCommand;

fn prepare_traced_child<I>(mut args: I) -> Command
    where I: std::iter::Iterator<Item=String> {

	let mut cmd = Command::new(args.next().expect("command argument missing!"));
	
	for a in args {
		cmd.arg(a);
	}
    unsafe {
        cmd.exec_before(|| ptrace::traceme());
    }
	cmd
}

fn wait_for_syscall(pid: nix::unistd::Pid) -> Option<u64> {

    ptrace::syscall(pid).unwrap();

    match wait::waitpid(pid, None) {
        Ok(WaitStatus::PtraceSyscall(pid)) => {
            let regs = ptrace::getregs(pid).unwrap();
            Some(regs.orig_rax)
        }
        Ok(WaitStatus::Exited(_, _)) |
        Ok(WaitStatus::Stopped(_, _)) => None,
        Ok(_) => None,
        Err(e) => panic!(e)
    }
}

fn handle_sys_openat(pid: nix::unistd::Pid) {
    use x64_openflags::x64_openflags::OpenFlags;

    let regs = ptrace::getregs(pid).unwrap();
    let filename = read_string(pid, regs.rsi);
    let flags = OpenFlags::from_bits_truncate(regs.rdx);
    //if flags.intersects(OpenFlags::O_WRONLY | OpenFlags::O_RDWR | OpenFlags::O_APPEND | OpenFlags::O_CREAT) {
    //    println!("> openat({}, '{}', {:?}, {})", regs.rdi, filename, flags, regs.r10);
    //}
    print!("> openat({}, '{}', {:?}, {})", regs.rdi, filename, flags, regs.r10);
}

fn handle_sys_connect(pid: nix::unistd::Pid) {
    let regs = ptrace::getregs(pid).unwrap();
    let (_typ, addr, port) = read_sockaddr(pid, regs.rsi, regs.rdx);
    // let host = dns_lookup::lookup_addr(&addr).unwrap();
    print!("> connect({}, {}:{}, {})", regs.rdi, addr, port, regs.rdx);
}

fn read_sockaddr(pid: nix::unistd::Pid, address: u64, len: u64) -> (i32, String, i16) {
    use byteorder::{LittleEndian, WriteBytesExt};

    let mut ret = vec![];

    // extract data, thanks to len
    // each loop read one word (8 bytes)
    for offset in 0..len / 8 {
        let mem = ptrace::read(pid, (address + (offset * 8) as u64) as *mut c_void).unwrap();
        ret.write_i64::<LittleEndian>(mem).unwrap();
    }

    // first parse as sockaddr in order to determine family
    let sockaddr: libc::sockaddr = unsafe { std::ptr::read(ret.as_ptr() as *const _) };
    match sockaddr.sa_family as i32 {
        libc::AF_INET => {
            let inet: libc::sockaddr_in = unsafe { std::ptr::read(ret.as_ptr() as *const _) };
            (libc::AF_INET,
             std::net::Ipv4Addr::from(inet.sin_addr.s_addr.to_be()).to_string(),
             inet.sin_port.to_be() as i16)
        },
        libc::AF_INET6 => {
            let inet6: libc::sockaddr_in6 = unsafe { std::ptr::read(ret.as_ptr() as *const _) };
            (libc::AF_INET6,
             std::net::Ipv6Addr::from(inet6.sin6_addr.s6_addr).to_string(),
             inet6.sin6_port.to_be() as i16)
        },
        libc::AF_UNIX => {
            let un: libc::sockaddr_un = unsafe { std::ptr::read(ret.as_ptr() as *const _) };
            let path: std::vec::Vec<u8> = un.sun_path.iter()
                                                     .map(|b| *b as u8)
                                                     .take_while(|b| *b != 0).collect();

            (libc::AF_UNIX,
             String::from_utf8_lossy(&path).to_string(),
             -1)
        },
        _ => panic!("not implemented")
    }
}

fn read_string(pid: nix::unistd::Pid, address: u64) -> String {
    use byteorder::{LittleEndian, WriteBytesExt};

    let mut ret = vec![];
    let mut offset: usize = 0;

    loop {
        let mem = ptrace::read(pid, (address + offset as u64) as *mut c_void).unwrap();
        ret.write_i64::<LittleEndian>(mem).unwrap();

        // break and truncate on string termination (null, 0)
        if let Some(nul) = ret.iter().position(|&b| b == 0) {
            ret.truncate(nul);
            break;
        }
        offset += 8;
    }
    ret.truncate(libc::PATH_MAX as usize);
    String::from_utf8_lossy(&ret).to_string()
}

fn handle_ret_value(pid: nix::unistd::Pid) {
    let regs = ptrace::getregs(pid).unwrap();
    let retval = match regs.rax as i64 {
        v if v < 0 => v + 1,
        v => v
    };
    println!(" = {}", retval);
}

fn main() {
	let mut child = prepare_traced_child(std::env::args().skip(1));
    let child = child.spawn().expect("failure in child process");
	let pid = nix::unistd::Pid::from_raw(child.id() as i32);

    ptrace::setoptions(pid, ptrace::Options::PTRACE_O_TRACESYSGOOD).unwrap();

    match wait::waitpid(pid, None) {
        Ok(WaitStatus::Stopped(_, signal::Signal::SIGTRAP)) => (),
        _ => panic!("child not stopped/initialized correctly")
    }

    while let Some(syscall) = wait_for_syscall(pid){

        // get syscall parameters
        match syscall {
            0 => print!("> read"),
            1 => print!("> write"),
            2 => print!("> open"),
            42 => handle_sys_connect(pid),
            257 => handle_sys_openat(pid),
            _ => continue
        };

        // return value from call
        match wait_for_syscall(pid){
            Some(_) => handle_ret_value(pid),
            None => break
        }
    }
    println!("Terminated...");
}
