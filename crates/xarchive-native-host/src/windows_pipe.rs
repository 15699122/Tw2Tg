//! Windows-only, deadline-bound Native bootstrap pipe I/O.
use interprocess::ConnectWaitMode;
use interprocess::os::windows::named_pipe::{PipeStream, pipe_mode};
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

pub type BytePipe = PipeStream<pipe_mode::Bytes, pipe_mode::Bytes>;
pub const PIPE_TIMEOUT: Duration = Duration::from_secs(3);

pub struct BoundedPipe {
    stream: BytePipe,
    deadline: Instant,
    stop: Arc<AtomicBool>,
}

impl BoundedPipe {
    pub fn new(stream: BytePipe, stop: Arc<AtomicBool>, deadline: Instant) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        Ok(Self {
            stream,
            deadline,
            stop,
        })
    }

    pub fn connect(endpoint: &Path) -> io::Result<Self> {
        if !endpoint.to_string_lossy().starts_with(r"\\.\pipe\") {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "bootstrap requires a local Named Pipe",
            ));
        }
        let deadline = Instant::now() + PIPE_TIMEOUT;
        let stream = BytePipe::connect_by_path_with_wait_mode(
            endpoint,
            ConnectWaitMode::Timeout(PIPE_TIMEOUT),
        )?;
        Self::new(stream, Arc::new(AtomicBool::new(false)), deadline)
    }

    fn check(&self) -> io::Result<()> {
        if self.stop.load(Ordering::Relaxed) {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionAborted,
                "pipe stopping",
            ));
        }
        if Instant::now() >= self.deadline {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "pipe deadline"));
        }
        Ok(())
    }

    /// Keep the response available until the exchange deadline, without
    /// the unbounded FlushFileBuffers wait used by PipeStream::flush/drop.
    pub fn finish_response(&mut self) {
        let _ = self.read(&mut [0u8; 1]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use interprocess::os::windows::named_pipe::PipeListenerOptions;

    #[test]
    fn stalled_reads_and_backpressured_writes_respect_the_deadline() {
        let endpoint = format!(r"\\.\pipe\xarchive-deadline-{}", std::process::id());
        let listener = PipeListenerOptions::new()
            .path(endpoint.as_str())
            .create_duplex::<pipe_mode::Bytes>()
            .unwrap();
        let client = BytePipe::connect_by_path_with_wait_mode(
            Path::new(&endpoint),
            ConnectWaitMode::Timeout(PIPE_TIMEOUT),
        )
        .unwrap();
        let server = listener.accept().unwrap();
        let mut pipe = BoundedPipe::new(
            server,
            Arc::new(AtomicBool::new(false)),
            Instant::now() + Duration::from_millis(80),
        )
        .unwrap();
        assert_eq!(
            pipe.read(&mut [0; 1]).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
        drop(pipe);
        drop(client);

        let client = BytePipe::connect_by_path_with_wait_mode(
            Path::new(&endpoint),
            ConnectWaitMode::Timeout(PIPE_TIMEOUT),
        )
        .unwrap();
        let server = listener.accept().unwrap();
        let started = Instant::now();
        let mut pipe = BoundedPipe::new(
            server,
            Arc::new(AtomicBool::new(false)),
            started + Duration::from_millis(80),
        )
        .unwrap();
        assert_eq!(
            pipe.write_all(&vec![0; 8 * 1024 * 1024])
                .unwrap_err()
                .kind(),
            io::ErrorKind::TimedOut
        );
        assert!(started.elapsed() < Duration::from_secs(1));
        drop(pipe);
        drop(client);
    }
}

impl Read for BoundedPipe {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        loop {
            self.check()?;
            match self.stream.read(buffer) {
                // PIPE_NOWAIT may report a zero-byte read before the peer
                // supplies data. One framed exchange waits for its deadline;
                // it must not interpret this as a complete empty request.
                Ok(0) => std::thread::sleep(Duration::from_millis(5)),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                result => return result,
            }
        }
    }
}
impl Write for BoundedPipe {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        loop {
            self.check()?;
            match self.stream.write(&buffer[..buffer.len().min(4096)]) {
                Ok(0) => std::thread::sleep(Duration::from_millis(5)),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                result => return result,
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        self.check()
    }
}
impl Drop for BoundedPipe {
    fn drop(&mut self) {
        // Deadline/shutdown is allowed to discard unread bytes; do not spawn
        // interprocess's unbounded limbo flush for a stalled peer.
        self.stream.assume_flushed();
    }
}
