use std::fs;
use std::io::{self, BufRead as _, BufReader, Write as _};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::Duration;

use gpui_kit::*;
use smol::io::{AsyncBufReadExt as _, AsyncWriteExt as _};

const HANDOFF_TIMEOUT: Duration = Duration::from_secs(2);

pub enum Launch {
    First(UnixListener),
    HandedOff,
    Standalone,
}

/// Makes this process the running Noty, or hands the launch to the one that
/// already runs so it can raise its window. The launcher's activation token
/// travels along because Wayland compositors only move focus with one.
pub fn claim() -> Launch {
    let token = std::env::var("XDG_ACTIVATION_TOKEN").ok();
    claim_at(&socket_path(), token.as_deref())
}

/// Calls `activate` with the launcher's activation token, if any, whenever
/// another launch hands over to this process.
pub fn serve(
    listener: UnixListener,
    cx: &mut App,
    activate: impl Fn(Option<String>, &mut App) + 'static,
) {
    let Ok(listener) = smol::Async::new(listener) else {
        return;
    };
    cx.spawn(async move |cx| {
        while let Ok((stream, _)) = listener.accept().await {
            let mut line = String::new();
            if smol::io::BufReader::new(&stream)
                .read_line(&mut line)
                .await
                .is_err()
            {
                continue;
            }
            let Some(token) = line.strip_prefix("activate") else {
                continue;
            };
            let token = Some(token.trim())
                .filter(|token| !token.is_empty())
                .map(str::to_owned);
            cx.update(|cx| activate(token, cx));
            (&stream).write_all(b"ok\n").await.ok();
        }
    })
    .detach();
}

fn socket_path() -> PathBuf {
    dirs::runtime_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join(format!("{}.sock", crate::APP_ID))
}

fn claim_at(path: &Path, token: Option<&str>) -> Launch {
    match UnixListener::bind(path) {
        Ok(listener) => return Launch::First(listener),
        Err(error) if error.kind() != io::ErrorKind::AddrInUse => return Launch::Standalone,
        Err(_) => {}
    }
    if hand_off(path, token).is_ok() {
        return Launch::HandedOff;
    }
    let _ = fs::remove_file(path);
    UnixListener::bind(path).map_or(Launch::Standalone, Launch::First)
}

fn hand_off(path: &Path, token: Option<&str>) -> io::Result<()> {
    let mut stream = UnixStream::connect(path)?;
    stream.set_read_timeout(Some(HANDOFF_TIMEOUT))?;
    writeln!(stream, "activate {}", token.unwrap_or_default())?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply)?;
    if reply.trim() == "ok" {
        Ok(())
    } else {
        Err(io::ErrorKind::InvalidData.into())
    }
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead as _, BufReader, Write as _};
    use std::os::unix::net::UnixListener;
    use std::thread;

    use super::{Launch, claim_at};

    fn answer_once(listener: UnixListener) -> thread::JoinHandle<String> {
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut line = String::new();
            BufReader::new(&stream).read_line(&mut line).unwrap();
            (&stream).write_all(b"ok\n").unwrap();
            line
        })
    }

    #[test]
    fn a_second_launch_hands_over_with_its_token() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("noty.sock");
        let Launch::First(listener) = claim_at(&path, None) else {
            panic!("the first launch should own the socket");
        };
        let running = answer_once(listener);
        assert!(matches!(
            claim_at(&path, Some("token-123")),
            Launch::HandedOff
        ));
        assert_eq!(running.join().unwrap(), "activate token-123\n");
    }

    #[test]
    fn a_socket_left_by_a_crash_is_taken_over() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("noty.sock");
        drop(UnixListener::bind(&path).unwrap());
        assert!(path.exists());
        assert!(matches!(claim_at(&path, None), Launch::First(_)));
    }

    #[test]
    fn an_instance_that_never_answers_is_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("noty.sock");
        let Launch::First(_silent) = claim_at(&path, None) else {
            panic!("the first launch should own the socket");
        };
        assert!(matches!(claim_at(&path, None), Launch::First(_)));
    }
}
