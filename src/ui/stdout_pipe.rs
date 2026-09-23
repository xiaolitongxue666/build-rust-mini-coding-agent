//! 第 12 课：把 `stdout` 接到管子上。`println!` 原样进视口，不用改几十处打印。
//! 界面画到**复制出来的原 stdout**，否则 Bubble Tea / crossterm 自己的画也会进管子，死循环。
//! 课上用 `tea.WithAltScreen()`；本仓库备用屏写到这份原句柄。

use std::fs::File;
use std::io::{self, PipeReader, PipeWriter};

pub struct StdoutCapture {
    original: File,
    reader: Option<PipeReader>,
    _writer: PipeWriter,
}

impl StdoutCapture {
    /// 先 `dup` 当前 stdout，再把 fd/句柄换成管子写端。
    pub fn install() -> io::Result<Self> {
        let original = dup_stdout()?;
        let (reader, writer) = io::pipe()?;
        redirect_stdout_to(&writer)?;
        Ok(Self {
            original,
            reader: Some(reader),
            _writer: writer,
        })
    }

    pub fn take_reader(&mut self) -> io::Result<PipeReader> {
        self.reader
            .take()
            .ok_or_else(|| io::Error::other("stdout pipe reader already taken"))
    }

    pub fn clone_original(&self) -> io::Result<File> {
        self.original.try_clone()
    }
}

impl Drop for StdoutCapture {
    fn drop(&mut self) {
        let _ = restore_stdout(&self.original);
    }
}

#[cfg(unix)]
fn dup_stdout() -> io::Result<File> {
    use std::os::fd::{FromRawFd, RawFd};
    let fd = unsafe { libc::dup(libc::STDOUT_FILENO) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(fd as RawFd) })
}

#[cfg(unix)]
fn redirect_stdout_to(writer: &PipeWriter) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    let rc = unsafe { libc::dup2(writer.as_raw_fd(), libc::STDOUT_FILENO) };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(unix)]
fn restore_stdout(original: &File) -> io::Result<()> {
    use std::os::fd::AsRawFd;
    let rc = unsafe { libc::dup2(original.as_raw_fd(), libc::STDOUT_FILENO) };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
fn dup_stdout() -> io::Result<File> {
    use std::os::windows::io::{FromRawHandle, OwnedHandle, RawHandle};
    let current = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };
    if current.is_null() || current == INVALID_HANDLE {
        return Err(io::Error::last_os_error());
    }
    let mut dup: RawHandle = std::ptr::null_mut();
    let proc = unsafe { GetCurrentProcess() };
    let ok = unsafe { DuplicateHandle(proc, current, proc, &mut dup, 0, 1, DUPLICATE_SAME_ACCESS) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    let handle = unsafe { OwnedHandle::from_raw_handle(dup) };
    Ok(File::from(handle))
}

#[cfg(windows)]
fn redirect_stdout_to(writer: &PipeWriter) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    let rc = unsafe { SetStdHandle(STD_OUTPUT_HANDLE, writer.as_raw_handle()) };
    if rc == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
fn restore_stdout(original: &File) -> io::Result<()> {
    use std::os::windows::io::AsRawHandle;
    let rc = unsafe { SetStdHandle(STD_OUTPUT_HANDLE, original.as_raw_handle()) };
    if rc == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(windows)]
const STD_OUTPUT_HANDLE: u32 = (-11_i32) as u32;
#[cfg(windows)]
const DUPLICATE_SAME_ACCESS: u32 = 2;
#[cfg(windows)]
const INVALID_HANDLE: *mut std::ffi::c_void = (-1isize) as *mut std::ffi::c_void;

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetStdHandle(n: u32) -> *mut std::ffi::c_void;
    fn SetStdHandle(n: u32, h: *mut std::ffi::c_void) -> i32;
    fn DuplicateHandle(
        src_proc: *mut std::ffi::c_void,
        src: *mut std::ffi::c_void,
        target_proc: *mut std::ffi::c_void,
        target: *mut *mut std::ffi::c_void,
        access: u32,
        inherit: i32,
        options: u32,
    ) -> i32;
    fn GetCurrentProcess() -> *mut std::ffi::c_void;
}
