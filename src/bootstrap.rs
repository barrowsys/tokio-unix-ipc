use serde_::de::DeserializeOwned;
use serde_::Serialize;
use std::cell::RefCell;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use tokio::net::UnixListener;

use crate::typed_channel::Sender;

/// A bootstrap helper.
///
/// This creates a unix socket that is linked to the file system so
/// that a [`Receiver`](struct.Receiver.html) can connect to it.  It
/// lets you send one or more messages to the connected receiver.
#[derive(Debug)]
pub struct Bootstrapper<T> {
    listener: UnixListener,
    sender: RefCell<Option<Sender<T>>>,
    path: PathBuf,
}

impl<T: Serialize + DeserializeOwned> Bootstrapper<T> {
    /// Creates a bootstrapper at a random socket in `/tmp`.
    pub fn new() -> io::Result<Bootstrapper<T>> {
        use rand::{thread_rng, RngCore};
        use std::time::{SystemTime, UNIX_EPOCH};

        let mut dir = std::env::temp_dir();
        let mut rng = thread_rng();
        let now = SystemTime::now();
        dir.push(&format!(
            ".rust-unix-ipc.{}-{}.sock",
            now.duration_since(UNIX_EPOCH).unwrap().as_secs(),
            rng.next_u64(),
        ));
        Bootstrapper::bind(&dir)
    }

    /// Creates a bootstrapper at a specific socket path.
    pub fn bind<P: AsRef<Path>>(p: P) -> io::Result<Bootstrapper<T>> {
        fs::remove_file(&p).ok();
        let listener = UnixListener::bind(&p)?;
        Ok(Bootstrapper {
            listener,
            sender: RefCell::new(None),
            path: p.as_ref().to_path_buf(),
        })
    }

    /// Returns the path of the socket.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Consumes the boostrapper and sends a single value in.
    ///
    /// This can be called multiple times to send more than one value
    /// into the inner socket.
    pub async fn send(&self, val: T) -> io::Result<()> {
        if self.sender.borrow().is_none() {
            let (sock, _) = self.listener.accept().await?;
            let sender = Sender::from_std(sock.into_std()?)?;
            *self.sender.borrow_mut() = Some(sender);
        }
        self.sender.borrow().as_ref().unwrap().send(val).await
    }
}

impl<T> Drop for Bootstrapper<T> {
    fn drop(&mut self) {
        fs::remove_file(&self.path).ok();
    }
}
