use sprobe::config::PrometheusConfig;
use sprobe::http::{HttpServerState, start_http_server};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};

#[test]
fn test_embedded_http_server_endpoints() {
    let config = PrometheusConfig {
        enabled: true,
        listen: "127.0.0.1:19101".to_string(),
        path: "/metrics".to_string(),
        allow_remote: false,
    };

    let state = HttpServerState {
        latest_snapshot: Arc::new(RwLock::new(None)),
        ready: Arc::new(AtomicBool::new(true)),
        warmup: Arc::new(AtomicBool::new(true)),
        commit: "test123".to_string(),
        wal_snapshots: Arc::new(RwLock::new(0u64)),
        wal_bytes: Arc::new(RwLock::new(0u64)),
    };

    let warmup_atomic = Arc::clone(&state.warmup);

    let handle =
        start_http_server(&config, "sprobe".to_string(), state).expect("HTTP 服务启动失败");

    // 等待监听就绪
    std::thread::sleep(std::time::Duration::from_millis(50));

    let client = reqwest::blocking::Client::new();

    // 1. 测试 /healthz
    let res = client.get("http://127.0.0.1:19101/healthz").send().unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.text().unwrap(), "OK\n");

    // 2. 测试 /readyz (此时处于 warmup 阶段，预期 503)
    let res = client.get("http://127.0.0.1:19101/readyz").send().unwrap();
    assert_eq!(res.status(), 503);

    // 解除 warmup
    warmup_atomic.store(false, Ordering::SeqCst);
    let res = client.get("http://127.0.0.1:19101/readyz").send().unwrap();
    assert_eq!(res.status(), 200);

    // 3. 测试 /status
    let res = client.get("http://127.0.0.1:19101/status").send().unwrap();
    assert_eq!(res.status(), 200);
    let text = res.text().unwrap();
    assert!(text.contains("\"ready\":true"));
    assert!(text.contains("\"warmup\":false"));

    // 4. 测试 /metrics (无快照时)
    let res = client.get("http://127.0.0.1:19101/metrics").send().unwrap();
    assert_eq!(res.status(), 200);
    let metrics_text = res.text().unwrap();
    assert!(metrics_text.contains("sprobe_agent_up 0"));
    assert!(metrics_text.contains("sprobe_agent_warmup 1"));

    // 5. 测试 404
    let res = client
        .get("http://127.0.0.1:19101/not_found")
        .send()
        .unwrap();
    assert_eq!(res.status(), 404);

    // 6. 测试 405 Method Not Allowed
    let res = client
        .post("http://127.0.0.1:19101/healthz")
        .send()
        .unwrap();
    assert_eq!(res.status(), 405);

    // 停止服务
    handle.stop();
}
