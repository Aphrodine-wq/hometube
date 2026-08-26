use anyhow::{Context, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_HEADER_BYTES: usize = 32 * 1024;
const MEDIA_PORT: u16 = 49_187;

pub struct MediaServer {
    port: u16,
    token: String,
    files: Arc<RwLock<HashMap<String, PathBuf>>>,
    last_cast_device_ip: RwLock<Option<String>>,
}

impl MediaServer {
    pub fn start() -> Result<Self> {
        let listener = TcpListener::bind(("0.0.0.0", MEDIA_PORT)).with_context(|| {
            format!("Could not start the local media stream on TCP port {MEDIA_PORT}")
        })?;
        let port = listener.local_addr()?.port();
        eprintln!("[hometube:media] listener_started address=0.0.0.0:{port}");
        let seed = format!(
            "{}:{}:{:?}",
            std::process::id(),
            port,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
        );
        let token = blake3::hash(seed.as_bytes()).to_hex().to_string();
        let files = Arc::new(RwLock::new(HashMap::new()));
        let worker_files = Arc::clone(&files);
        let worker_token = token.clone();

        std::thread::Builder::new()
            .name("hometube-media-listener".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    let Ok(stream) = stream else { continue };
                    let files = Arc::clone(&worker_files);
                    let token = worker_token.clone();
                    let _ = std::thread::Builder::new()
                        .name("hometube-media-request".into())
                        .spawn(move || {
                            if let Err(error) = handle_request(stream, &token, &files) {
                                if !matches!(
                                    error.kind(),
                                    io::ErrorKind::ConnectionReset
                                        | io::ErrorKind::ConnectionAborted
                                        | io::ErrorKind::BrokenPipe
                                ) {
                                    eprintln!("[hometube:media] request_failed error={error}");
                                }
                            }
                        });
                }
            })
            .context("Could not start the local media listener")?;

        Ok(Self {
            port,
            token,
            files,
            last_cast_device_ip: RwLock::new(None),
        })
    }

    pub fn url_for(&self, media_id: &str, path: &Path) -> Result<String> {
        self.register(media_id, path)?;
        Ok(format!(
            "http://127.0.0.1:{}/media/{}/{}",
            self.port, self.token, media_id
        ))
    }

    pub fn url_for_cast(&self, media_id: &str, path: &Path, device_ip: &str) -> Result<String> {
        self.register(media_id, path)?;
        *self.last_cast_device_ip.write() = Some(device_ip.to_owned());
        let local_ip = local_ip_for(device_ip)?;
        eprintln!(
            "[hometube:cast] media_registered media_id={media_id} address={local_ip}:{}",
            self.port
        );
        Ok(format!(
            "http://{local_ip}:{}/media/{}/{}",
            self.port, self.token, media_id
        ))
    }

    pub fn last_cast_device_ip(&self) -> Option<String> {
        self.last_cast_device_ip.read().clone()
    }

    pub fn unregister(&self, media_id: &str) {
        self.files.write().remove(media_id);
    }

    fn register(&self, media_id: &str, path: &Path) -> Result<()> {
        let path = path
            .canonicalize()
            .with_context(|| format!("Media file does not exist: {}", path.display()))?;
        if !path.is_file() {
            anyhow::bail!("Media path is not a file: {}", path.display());
        }
        self.files.write().insert(media_id.to_owned(), path);
        Ok(())
    }
}

fn local_ip_for(device_ip: &str) -> Result<std::net::IpAddr> {
    let device: std::net::IpAddr = device_ip.parse().context("Invalid Cast device address")?;
    let bind = if device.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    };
    let socket = std::net::UdpSocket::bind(bind)?;
    socket.connect((device, 9))?;
    Ok(socket.local_addr()?.ip())
}

fn handle_request(
    mut stream: TcpStream,
    token: &str,
    files: &RwLock<HashMap<String, PathBuf>>,
) -> io::Result<()> {
    let peer = stream.peer_addr().ok();
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let request = read_headers(&mut stream)?;
    let request = String::from_utf8_lossy(&request);
    let mut lines = request.split("\r\n");
    let mut request_line = lines.next().unwrap_or_default().split_whitespace();
    let method = request_line.next().unwrap_or_default();
    let target = request_line.next().unwrap_or_default();

    if method == "OPTIONS" {
        return write_empty(&mut stream, "204 No Content", &[]);
    }
    if method != "GET" && method != "HEAD" {
        return write_empty(
            &mut stream,
            "405 Method Not Allowed",
            &[("Allow", "GET, HEAD, OPTIONS")],
        );
    }

    let prefix = format!("/media/{token}/");
    let media_id = target
        .split('?')
        .next()
        .and_then(|path| path.strip_prefix(&prefix));
    let Some(media_id) = media_id.filter(|id| !id.is_empty() && !id.contains('/')) else {
        return write_empty(&mut stream, "404 Not Found", &[]);
    };
    let Some(path) = files.read().get(media_id).cloned() else {
        return write_empty(&mut stream, "404 Not Found", &[]);
    };

    let mut file = File::open(&path)?;
    let length = file.metadata()?.len();
    let range_header = lines.find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("range").then(|| value.trim())
    });
    if !peer.is_some_and(|address| address.ip().is_loopback()) {
        eprintln!(
            "[hometube:media] request peer={} method={method} media_id={media_id} range={}",
            peer.map(|address| address.ip().to_string())
                .unwrap_or_else(|| "unknown".into()),
            range_header.unwrap_or("none")
        );
    }
    let requested_range = match range_header {
        Some(value) => match parse_range(value, length) {
            Some(range) => Some(range),
            None => {
                let content_range = format!("bytes */{length}");
                return write_empty(
                    &mut stream,
                    "416 Range Not Satisfiable",
                    &[("Content-Range", &content_range)],
                );
            }
        },
        None => None,
    };
    let (start, end, status) = requested_range
        .map(|(start, end)| (start, end, "206 Partial Content"))
        .unwrap_or((0, length.saturating_sub(1), "200 OK"));
    let response_length = if length == 0 { 0 } else { end - start + 1 };
    let content_type = content_type(&path);

    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nAccept-Ranges: bytes\r\nContent-Length: {response_length}\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-store\r\nConnection: close\r\n"
    )?;
    if requested_range.is_some() {
        write!(stream, "Content-Range: bytes {start}-{end}/{length}\r\n")?;
    }
    write!(stream, "\r\n")?;

    if method == "GET" && response_length > 0 {
        file.seek(SeekFrom::Start(start))?;
        let mut reader = BufReader::with_capacity(128 * 1024, file).take(response_length);
        io::copy(&mut reader, &mut stream)?;
    }
    Ok(())
}

fn read_headers(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut request = Vec::with_capacity(2048);
    let mut buffer = [0_u8; 2048];
    while request.len() < MAX_HEADER_BYTES {
        let read = stream.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        request.extend_from_slice(&buffer[..read]);
        if request.windows(4).any(|window| window == b"\r\n\r\n") {
            return Ok(request);
        }
    }
    if request.len() >= MAX_HEADER_BYTES {
        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "request headers are too large",
        ))
    } else {
        Ok(request)
    }
}

fn write_empty(stream: &mut TcpStream, status: &str, extra: &[(&str, &str)]) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Length: 0\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n"
    )?;
    for (name, value) in extra {
        write!(stream, "{name}: {value}\r\n")?;
    }
    write!(stream, "\r\n")
}

fn parse_range(value: &str, length: u64) -> Option<(u64, u64)> {
    if length == 0 {
        return None;
    }
    let value = value.strip_prefix("bytes=")?;
    if value.contains(',') {
        return None;
    }
    let (start, end) = value.split_once('-')?;
    if start.is_empty() {
        let suffix = end.parse::<u64>().ok()?.min(length);
        return (suffix > 0).then_some((length - suffix, length - 1));
    }
    let start = start.parse::<u64>().ok()?;
    if start >= length {
        return None;
    }
    let end = if end.is_empty() {
        length - 1
    } else {
        end.parse::<u64>().ok()?.min(length - 1)
    };
    (start <= end).then_some((start, end))
}

fn content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "avi" => "video/x-msvideo",
        "mpeg" | "mpg" => "video/mpeg",
        "ts" | "m2ts" => "video/mp2t",
        "ogv" | "ogg" => "video/ogg",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::parse_range;

    #[test]
    fn parses_common_byte_ranges() {
        assert_eq!(parse_range("bytes=0-99", 1_000), Some((0, 99)));
        assert_eq!(parse_range("bytes=900-", 1_000), Some((900, 999)));
        assert_eq!(parse_range("bytes=-100", 1_000), Some((900, 999)));
        assert_eq!(parse_range("bytes=0-9999", 1_000), Some((0, 999)));
    }

    #[test]
    fn rejects_invalid_or_multiple_ranges() {
        assert_eq!(parse_range("bytes=1000-", 1_000), None);
        assert_eq!(parse_range("bytes=20-10", 1_000), None);
        assert_eq!(parse_range("bytes=0-1,4-5", 1_000), None);
        assert_eq!(parse_range("items=0-1", 1_000), None);
    }
}
