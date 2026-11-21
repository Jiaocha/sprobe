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
    let port = 9977;
    // 启动本地 mock ingest
    let child = Command::new("node")
        .arg("scripts/mock_ingest.mjs")
        .arg(port.to_string())
        .spawn()
        .expect("无法启动 scripts/mock_ingest.mjs");

    let _guard = MockServerGuard { child };

    // 等待服务启动就绪
    std::thread::sleep(Duration::from_millis(600));

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
