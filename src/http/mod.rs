use crate::config::PrometheusConfig;
use crate::output::prometheus::{PrometheusContext, encode_prometheus};
use crate::snapshot::Snapshot;
use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

#[derive(Clone)]
pub struct HttpServerState {
    pub latest_snapshot: Arc<RwLock<Option<Snapshot>>>,
    pub ready: Arc<AtomicBool>,
    pub warmup: Arc<AtomicBool>,
    pub commit: String,
    pub wal_snapshots: Arc<RwLock<u64>>,
    pub wal_bytes: Arc<RwLock<u64>>,
}

pub struct HttpServerHandle {
    shutdown: Arc<AtomicBool>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl HttpServerHandle {
    pub fn stop(mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

pub fn start_http_server(
    config: &PrometheusConfig,
    namespace: String,
    state: HttpServerState,
) -> std::io::Result<HttpServerHandle> {
    let listener = TcpListener::bind(&config.listen)?;
    listener.set_nonblocking(true)?;

    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = Arc::clone(&shutdown);
    let path = config.path.clone();
    let allow_remote = config.allow_remote;

    let thread_handle = std::thread::spawn(move || {
        while !shutdown_clone.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((stream, peer_addr)) => {
                    handle_connection(stream, peer_addr, &path, allow_remote, &namespace, &state);
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        }
    });

    Ok(HttpServerHandle {
        shutdown,
        thread_handle: Some(thread_handle),
    })
}

fn handle_connection(
    mut stream: TcpStream,
    peer_addr: SocketAddr,
    metrics_path: &str,
    allow_remote: bool,
    namespace: &str,
    state: &HttpServerState,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));

    // 检查远程访问安全性：默认只允许回环访问
    if !allow_remote && !peer_addr.ip().is_loopback() {
        let resp =
            "HTTP/1.1 403 Forbidden\r\nContent-Length: 9\r\nConnection: close\r\n\r\nForbidden";
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() || request_line.is_empty() {
        return;
    }

    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        let resp = "HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    let method = parts[0];
    let req_path = parts[1];

    if method != "GET" && method != "HEAD" {
        let resp =
            "HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
        return;
    }

    if req_path == metrics_path {
        let snap_guard = state.latest_snapshot.read().unwrap();
        let wal_snaps = *state.wal_snapshots.read().unwrap();
        let wal_b = *state.wal_bytes.read().unwrap();
        let ctx = PrometheusContext {
            namespace: namespace.to_string(),
            commit: state.commit.clone(),
            wal_snapshots: wal_snaps,
            wal_bytes: wal_b,
        };
        let body = encode_prometheus(snap_guard.as_ref(), &ctx);
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
    } else if req_path == "/healthz" {
        let body = "OK\n";
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
    } else if req_path == "/readyz" {
        let is_ready = state.ready.load(Ordering::SeqCst);
        let is_warmup = state.warmup.load(Ordering::SeqCst);
        if is_ready && !is_warmup {
            let body = "OK\n";
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes());
        } else {
            let body = "Service Unavailable: Agent warming up or starting\n";
            let resp = format!(
                "HTTP/1.1 503 Service Unavailable\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(resp.as_bytes());
        }
    } else if req_path == "/status" {
        let snap_guard = state.latest_snapshot.read().unwrap();
        let (seq, health) = if let Some(ref s) = *snap_guard {
            (s.sequence, format!("{:?}", s.health.status))
        } else {
            (0, "unknown".to_string())
        };
        let body = format!(
            "{{\"ready\":{},\"warmup\":{},\"sequence\":{},\"health\":\"{}\"}}\n",
            state.ready.load(Ordering::SeqCst),
            state.warmup.load(Ordering::SeqCst),
            seq,
            health
        );
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
    } else {
        let resp = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
        let _ = stream.write_all(resp.as_bytes());
    }
}
