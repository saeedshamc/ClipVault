//! Local IPC transport: named pipe (Windows) / Unix domain socket (Linux).

use std::io::{Read, Write};

use anyhow::{anyhow, Context, Result};
use clipvault_core::ipc::{decode_message, encode_message, Request, Response};
use interprocess::local_socket::{prelude::*, ListenerOptions};
#[cfg(unix)]
use interprocess::local_socket::{GenericFilePath, ToFsName};
#[cfg(windows)]
use interprocess::local_socket::{GenericNamespaced, ToNsName};

/// Endpoint name for the current user (namespaced pipe on Windows, filesystem path on Unix).
pub fn endpoint_name() -> Result<String> {
    #[cfg(windows)]
    {
        let user = std::env::var("USERNAME").unwrap_or_else(|_| "user".into());
        Ok(format!("clipvault-{user}"))
    }
    #[cfg(unix)]
    {
        use clipvault_core::{AppPaths, Config};
        let paths =
            AppPaths::resolve(&Config::default()).map_err(|err| anyhow!(err.to_string()))?;
        std::fs::create_dir_all(&paths.state_dir)?;
        Ok(paths
            .state_dir
            .join("clipvault.sock")
            .to_string_lossy()
            .into_owned())
    }
    #[cfg(not(any(windows, unix)))]
    {
        Err(anyhow!("IPC unsupported on this platform"))
    }
}

/// Send one request and wait for one response.
pub fn call(req: &Request) -> Result<Response> {
    let mut stream = connect()?;
    let bytes = encode_message(req).map_err(|err| anyhow!(err))?;
    stream.write_all(&bytes)?;
    stream.flush()?;

    let mut buf = vec![0_u8; 4];
    stream.read_exact(&mut buf)?;
    let len = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    buf.resize(4 + len, 0);
    stream.read_exact(&mut buf[4..])?;
    let (resp, _) = decode_message::<Response>(&buf).map_err(|err| anyhow!(err))?;
    Ok(resp)
}

fn connect() -> Result<LocalSocketStream> {
    let name = endpoint_name()?;
    #[cfg(windows)]
    {
        let ns_name = name
            .as_str()
            .to_ns_name::<GenericNamespaced>()
            .map_err(|err| anyhow!(err.to_string()))?;
        LocalSocketStream::connect(ns_name).with_context(|| format!("connect named pipe {name}"))
    }
    #[cfg(unix)]
    {
        let fs_name = name
            .as_str()
            .to_fs_name::<GenericFilePath>()
            .map_err(|err| anyhow!(err.to_string()))?;
        LocalSocketStream::connect(fs_name).with_context(|| format!("connect socket {name}"))
    }
}

/// Accept loop for IPC clients.
pub fn serve_forever<F>(mut handler: F) -> Result<()>
where
    F: FnMut(Request) -> Response + Send + 'static,
{
    let name = endpoint_name()?;
    #[cfg(windows)]
    let listener = {
        let ns_name = name
            .as_str()
            .to_ns_name::<GenericNamespaced>()
            .map_err(|err| anyhow!(err.to_string()))?;
        ListenerOptions::new()
            .name(ns_name)
            .create_sync()
            .with_context(|| format!("listen named pipe {name}"))?
    };
    #[cfg(unix)]
    let listener = {
        let _ = std::fs::remove_file(&name);
        let fs_name = name
            .as_str()
            .to_fs_name::<GenericFilePath>()
            .map_err(|err| anyhow!(err.to_string()))?;
        let listener = ListenerOptions::new()
            .name(fs_name)
            .create_sync()
            .with_context(|| format!("listen socket {name}"))?;
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            let _ = std::fs::set_permissions(&name, perms);
        }
        listener
    };

    tracing::info!(endpoint = %name, "IPC listening");

    loop {
        let mut stream = match listener.accept() {
            Ok(s) => s,
            Err(err) => {
                tracing::warn!(error = %err, "accept failed");
                continue;
            }
        };
        if let Err(err) = handle_client(&mut stream, &mut handler) {
            tracing::warn!(error = %err, "IPC client error");
        }
    }
}

fn handle_client<F>(stream: &mut LocalSocketStream, handler: &mut F) -> Result<()>
where
    F: FnMut(Request) -> Response,
{
    let mut header = [0_u8; 4];
    stream.read_exact(&mut header)?;
    let len = u32::from_le_bytes(header) as usize;
    if len > 16 * 1024 * 1024 {
        return Err(anyhow!("IPC message too large: {len}"));
    }
    let mut body = vec![0_u8; len];
    stream.read_exact(&mut body)?;
    let mut full = Vec::with_capacity(4 + len);
    full.extend_from_slice(&header);
    full.extend_from_slice(&body);
    let (req, _) = decode_message::<Request>(&full).map_err(|err| anyhow!(err))?;
    let resp = handler(req);
    let out = encode_message(&resp).map_err(|err| anyhow!(err))?;
    stream.write_all(&out)?;
    stream.flush()?;
    Ok(())
}
