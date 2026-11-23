//! Minimal audited Win32 Job Object I/O Completion Port wrapper.
//!
//! Kept isolated in its own tiny crate so the rest of the workspace retains
//! `unsafe_code = "forbid"`.

#![cfg(windows)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::c_void;
use std::io;
use std::mem::size_of;
use std::ptr::null_mut;
use std::time::{Duration, Instant};

use firehazard::handle::AsLocalHandle;
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE, WAIT_TIMEOUT};
use windows_sys::Win32::System::IO::{CreateIoCompletionPort, GetQueuedCompletionStatus};
use windows_sys::Win32::System::JobObjects::{
    JOBOBJECT_ASSOCIATE_COMPLETION_PORT, JobObjectAssociateCompletionPortInformation,
    SetInformationJobObject,
};

const JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO: u32 = 4;

/// Owned Windows I/O completion port attached to one Job Object.
pub struct JobCompletionPort {
    port: HANDLE,
    completion_key: usize,
}

// SAFETY: A Windows I/O completion port HANDLE is kernel-managed and safe to
// transfer or query concurrently across threads; `Drop` closes it once.
unsafe impl Send for JobCompletionPort {}
// SAFETY: `GetQueuedCompletionStatus` on a valid IOCP handle is thread-safe.
unsafe impl Sync for JobCompletionPort {}

impl Drop for JobCompletionPort {
    fn drop(&mut self) {
        // SAFETY: `self.port` is a unique owned non-null IOCP handle created by `attach`.
        unsafe {
            let _ = CloseHandle(self.port);
        }
    }
}

impl JobCompletionPort {
    /// Creates a new I/O completion port and associates it with `job`.
    ///
    /// Note that Windows permits associating a Job Object with a completion port
    /// at most once in the Job's lifetime, so callers should attach lazily when
    /// beginning a drain (allowing a crash recovery successor to attach its own).
    pub fn attach(job: &firehazard::job::OwnedHandle) -> io::Result<Self> {
        let job_handle: *mut c_void = job.as_handle().cast();
        // SAFETY: `INVALID_HANDLE_VALUE` with null existing port creates a standalone IOCP.
        let port = unsafe { CreateIoCompletionPort(INVALID_HANDLE_VALUE, null_mut(), 0, 1) };
        if port.is_null() {
            return Err(io::Error::last_os_error());
        }
        let iocp = Self {
            port,
            completion_key: job_handle as usize,
        };
        let assoc = JOBOBJECT_ASSOCIATE_COMPLETION_PORT {
            CompletionKey: job_handle,
            CompletionPort: iocp.port,
        };
        // SAFETY: `job_handle` is a valid Job handle borrowed from `&OwnedHandle` and
        // `assoc` matches `JobObjectAssociateCompletionPortInformation`.
        let ok = unsafe {
            SetInformationJobObject(
                job_handle,
                JobObjectAssociateCompletionPortInformation,
                (&raw const assoc).cast::<c_void>(),
                size_of::<JOBOBJECT_ASSOCIATE_COMPLETION_PORT>() as u32,
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(iocp)
    }

    /// Blocks until the attached Job reports `JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO`
    /// or `timeout` elapses.
    pub fn wait_active_process_zero(&self, timeout: Duration) -> io::Result<()> {
        let deadline = Instant::now()
            .checked_add(timeout)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid drain timeout"))?;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let timeout_ms = u32::try_from(remaining.as_millis()).unwrap_or(u32::MAX);
            let mut message_id: u32 = 0;
            let mut completion_key: usize = 0;
            let mut overlapped = null_mut();
            // SAFETY: `self.port` is a live IOCP handle and all out-pointers are valid.
            let ok = unsafe {
                GetQueuedCompletionStatus(
                    self.port,
                    &raw mut message_id,
                    &raw mut completion_key,
                    &raw mut overlapped,
                    timeout_ms,
                )
            };
            if ok != 0 {
                if completion_key == self.completion_key
                    && message_id == JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO
                {
                    return Ok(());
                }
                if Instant::now() >= deadline {
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "timed out waiting for JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO",
                    ));
                }
                continue;
            }
            let error = io::Error::last_os_error();
            if error.raw_os_error() == Some(WAIT_TIMEOUT as i32) {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "timed out waiting for JOB_OBJECT_MSG_ACTIVE_PROCESS_ZERO",
                ));
            }
            return Err(error);
        }
    }
}
