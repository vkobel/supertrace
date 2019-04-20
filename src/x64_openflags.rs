pub mod x64_openflags {

    use bitflags::bitflags;

    bitflags! {
        pub struct OpenFlags: u64 {
            const O_RDONLY = 0;
            const O_WRONLY = 1;
            const O_RDWR = 2;
            const O_CLOEXEC = 0x80000;
            const O_TRUNC = 512;
            const O_APPEND = 1024;
            const O_CREAT = 64;
            const O_EXCL = 128;
            const O_NOCTTY = 256;
            const O_NONBLOCK = 2048;
            const O_SYNC = 1052672;
            const O_RSYNC = 1052672;
            const O_DSYNC = 4096;
            const O_FSYNC = 0x101000;
            const O_NOATIME = 0o1000000;
            const O_PATH = 0o10000000;
            const O_ASYNC = 0x2000;
            const O_NDELAY = 0x800;
            const O_DIRECT = 0x4000;
            const O_DIRECTORY = 0x10000;
            const O_NOFOLLOW = 0x20000;
            const O_TMPFILE = 0o20000000 | 0x4000;
        }
    }
}
