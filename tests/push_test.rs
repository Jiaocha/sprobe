use chrono::Utc;
use sprobe::config::PushConfig;
use sprobe::output::{IngestBatch, PushClient};
use sprobe::snapshot::Snapshot;
use std::process::{Child, Command};
use std::time::Duration;
use uuid::Uuid;

struct MockServerGuard {
    child: Child,
}

impl Drop for MockServerGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[test]
fn test_push_batch_to_mock_ingest() {
    // 动态选取随机可用端口
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("无法绑定临时端口");
    let port = listener.local_addr().unwrap().port();
    drop(listener); // 释放端口供 node 使用

    // 启动本地 mock ingest
    let child = match Command::new("node")
        .arg("scripts/mock_ingest.mjs")
        .arg(port.to_string())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[SKIP] 系统未检测到 node 环境或执行失败: {e}");
            return;
        }
    };

    let _guard = MockServerGuard { child };

    // 轮询等待服务监听就绪，最多等待 5 秒
    let start_wait = std::time::Instant::now();
    let mut server_ready = false;
    while start_wait.elapsed() < Duration::from_secs(5) {
        if std::net::TcpStream::connect(format!("127.0.0.1:{port}")).is_ok() {
            server_ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    if !server_ready {
        eprintln!("[SKIP] Mock 服务在 5 秒内未就绪，跳过测试");
        return;
    }

    // 读取黄金快照
    let fixture_str = include_str!("fixtures/golden_snapshot_v1_0.json");
    let golden_snapshot: Snapshot = serde_json::from_str(fixture_str).unwrap();

    let push_config = PushConfig {
        endpoint: format!("http://127.0.0.1:{port}/api/v1/ingest"),
        gzip: true,
        allow_insecure: true,
        tls_insecure_skip_verify: true,
        ..Default::default()
    };

    let client = PushClient::new(&push_config, Some("test-bearer-token".to_string()))
        .expect("初始化 PushClient 失败");

    let batch = IngestBatch {
        schema_version: "1.0".to_string(),
        batch_id: Uuid::new_v4(),
        agent_id: golden_snapshot.agent.agent_id,
        instance_id: golden_snapshot.agent.instance_id,
        created_at: Utc::now(),
        sequence_first: 100,
        sequence_last: 100,
        snapshot_count: 1,
        snapshots: vec![golden_snapshot],
    };

    // 1. 首次推送验证
    let res = client.send_batch(&batch).expect("首次推送请求失败");
    assert_eq!(res.status, "accepted");
    assert_eq!(res.accepted, 1);
    assert_eq!(res.duplicates, 0);

    // 2. 幂等重复推送验证
    let second_res = client.send_batch(&batch).expect("二次推送请求失败");
    assert_eq!(second_res.status, "accepted");
    assert_eq!(second_res.accepted, 0);
    assert_eq!(second_res.duplicates, 1);
}
