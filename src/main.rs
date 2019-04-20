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
        //cmd.exec_before(|| nix::sys::signal::kill(nix::unistd::Pid::this(), signal::Signal::SIGSTOP));
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

fn handle_sys_read(pid: nix::unistd::Pid) {
    let regs = ptrace::getregs(pid).unwrap();
    print!("> read({}, 0x{:x}, {})", regs.rdi, regs.rsi, regs.rdx);
}

fn handle_sys_openat(pid: nix::unistd::Pid) {
    use x64_openflags::x64_openflags::OpenFlags;

    let regs = ptrace::getregs(pid).unwrap();
    let filename = read_string(pid, regs.rsi);
    let flags = OpenFlags::from_bits_truncate(regs.rdx);
    print!("> openat({}, '{}', {:?}, {})", regs.rdi, filename, flags, regs.r10);
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
            //0 => handle_sys_read(pid),
            //1 => print!("> write"),
            2 => print!("> open"),
            257 => handle_sys_openat(pid),
            _ => {
                continue
            }
        };
        
        // return value from call
        match wait_for_syscall(pid){
            Some(_) => handle_ret_value(pid),
            None => break
        }

    }

    println!("Terminated...");
}
