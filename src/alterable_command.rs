pub mod alterable_command {

    use std::io;
    use std::process::Command;
    use std::os::unix::process::CommandExt;

    pub trait AlterableCommand {
        unsafe fn exec_before<F>(&mut self, f: F) -> &Self
            where F: Fn() -> nix::Result<()> + Send + Sync + 'static;
    }

    impl AlterableCommand for Command {
        unsafe fn exec_before<F>(&mut self, f: F) -> &Self
            where F: Fn() -> nix::Result<()> + Send + Sync + 'static {
            
            self.pre_exec(move || {
                match f() {
                    Ok(()) => Ok(()),
                    Err(nix::Error::Sys(errno)) => Err(io::Error::from_raw_os_error(errno as i32)),
                    Err(e) => Err(io::Error::new(io::ErrorKind::Other, e))
                }
            });
            self
        }
    }
}
