use serde_json::json;
use sprobe::snapshot::Snapshot;
use std::fs;
use std::path::Path;

fn load_contract_schema() -> serde_json::Value {
    let schema_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("contract/sprobe_contract_v1_0.schema.json");
    let content = fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("无法读取契约文件 {:?}: {}", schema_path, e));
    serde_json::from_str(&content).unwrap_or_else(|e| panic!("契约文件不是合法 JSON: {}", e))
}

fn get_subschema_validator(
    root_schema: &serde_json::Value,
    def_name: &str,
) -> jsonschema::Validator {
    let mut schema = root_schema.clone();
    if let Some(obj) = schema.as_object_mut() {
        obj.insert(
            "$ref".to_string(),
            serde_json::Value::String(format!("#/$defs/{}", def_name)),
        );
    }
    jsonschema::validator_for(&schema)
        .unwrap_or_else(|e| panic!("编译定义 #/$defs/{} 失败: {}", def_name, e))
}

#[test]
fn test_schema_compiles_successfully() {
    let schema_json = load_contract_schema();
    let _ = jsonschema::validator_for(&schema_json).expect("根契约 Schema 编译失败");
}

#[test]
fn test_golden_snapshot_contract() {
    let root_schema = load_contract_schema();
    let snapshot_validator = get_subschema_validator(&root_schema, "Snapshot");

    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden_snapshot_v1_0.json");
    let fixture_str = fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("无法读取黄金快照文件 {:?}: {}", fixture_path, e));
    let golden_snapshot: serde_json::Value = serde_json::from_str(&fixture_str)
        .unwrap_or_else(|e| panic!("黄金快照不是有效 JSON: {}", e));

    let errors: Vec<String> = snapshot_validator
        .iter_errors(&golden_snapshot)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "黄金快照应该校验通过，但出现错误: {:?}",
        errors
    );
}

#[test]
fn test_rust_snapshot_model_roundtrip() {
    let root_schema = load_contract_schema();
    let snapshot_validator = get_subschema_validator(&root_schema, "Snapshot");

    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden_snapshot_v1_0.json");
    let fixture_str = fs::read_to_string(&fixture_path).unwrap();

    // 1. 反序列化为 Rust 结构体 Snapshot
    let snapshot: Snapshot = serde_json::from_str(&fixture_str)
        .unwrap_or_else(|e| panic!("黄金样本反序列化为 Rust Snapshot 模型失败: {}", e));

    // 2. 将结构体重新序列化为 JSON Value
    let serialized =
        serde_json::to_value(&snapshot).unwrap_or_else(|e| panic!("Snapshot 序列化失败: {}", e));

    // 3. 校验重新序列化后的 JSON 依然完全合规
    let errors: Vec<String> = snapshot_validator
        .iter_errors(&serialized)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "Rust 模型重新序列化后校验失败: {:?}",
        errors
    );
}

#[test]
fn test_ingest_batch_contract() {
    let root_schema = load_contract_schema();
    let batch_validator = get_subschema_validator(&root_schema, "IngestBatch");

    let fixture_path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden_snapshot_v1_0.json");
    let fixture_str = fs::read_to_string(&fixture_path)
        .unwrap_or_else(|e| panic!("无法读取黄金快照文件 {:?}: {}", fixture_path, e));
    let golden_snapshot: serde_json::Value = serde_json::from_str(&fixture_str).unwrap();

    let valid_batch = json!({
        "schema_version": "1.0",
        "batch_id": "00000000-0000-0000-0000-000000000099",
        "agent_id": "6f1c2d3e-4a5b-4c6d-8e9f-0a1b2c3d4e5f",
        "instance_id": "9a8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d",
        "created_at": "2026-11-21T01:21:00.000Z",
        "sequence_first": 123456,
        "sequence_last": 123456,
        "snapshot_count": 1,
        "snapshots": [golden_snapshot]
    });

    let errors: Vec<String> = batch_validator
        .iter_errors(&valid_batch)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "合规 IngestBatch 应该校验通过，但出现错误: {:?}",
        errors
    );

    // 非法批次：snapshot_count 为 0
    let invalid_count_batch = json!({
        "schema_version": "1.0",
        "batch_id": "00000000-0000-0000-0000-000000000099",
        "agent_id": "6f1c2d3e-4a5b-4c6d-8e9f-0a1b2c3d4e5f",
        "instance_id": "9a8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d",
        "created_at": "2026-11-21T01:21:00.000Z",
        "sequence_first": 123456,
        "sequence_last": 123456,
        "snapshot_count": 0,
        "snapshots": []
    });
    assert!(
        !batch_validator.is_valid(&invalid_count_batch),
        "空批次应该校验失败"
    );
}

#[test]
fn test_error_response_contract() {
    let root_schema = load_contract_schema();
    let error_validator = get_subschema_validator(&root_schema, "ErrorResponse");

    let valid_error = json!({
        "error": {
            "code": "validation_failed",
            "message": "快照缺失必需字段",
            "retryable": false,
            "request_id": "req_123456",
            "server_time": "2026-11-21T05:00:00Z"
        }
    });

    let errors: Vec<String> = error_validator
        .iter_errors(&valid_error)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "合法 ErrorResponse 应该校验通过，但出现错误: {:?}",
        errors
    );

    // 非法错误体：code 不在枚举中
    let invalid_code = json!({
        "error": {
            "code": "invalid_code_not_in_enum",
            "message": "未知错误",
            "retryable": false,
            "request_id": "req_123456",
            "server_time": "2026-11-21T05:00:00Z"
        }
    });
    assert!(
        !error_validator.is_valid(&invalid_code),
        "非法 code 应该校验失败"
    );
}

#[test]
fn test_heartbeat_contract() {
    let root_schema = load_contract_schema();
    let heartbeat_validator = get_subschema_validator(&root_schema, "HeartbeatRequest");

    let valid_heartbeat = json!({
        "schema_version": "1.0",
        "agent_id": "00000000-0000-0000-0000-000000000001",
        "instance_id": "00000000-0000-0000-0000-000000000002",
        "sent_at": "2026-11-21T05:00:00Z",
        "uptime_seconds": 3600.5,
        "ready": true,
        "warmup": false,
        "health_status": "ok",
        "health_status_code": 0,
        "last_sequence": 42,
        "wal_snapshots": 0
    });

    let errors: Vec<String> = heartbeat_validator
        .iter_errors(&valid_heartbeat)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "合法 HeartbeatRequest 应该校验通过，但出现错误: {:?}",
        errors
    );
}

#[test]
fn test_ingest_success_response_contract() {
    let root_schema = load_contract_schema();
    let resp_validator = get_subschema_validator(&root_schema, "IngestSuccessResponse");

    let valid_resp = json!({
        "schema_version": "1.0",
        "status": "accepted",
        "batch_id": "00000000-0000-0000-0000-000000000003",
        "accepted": 6,
        "duplicates": 0,
        "rejected": [],
        "server_time": "2026-11-21T05:00:00Z"
    });

    let errors: Vec<String> = resp_validator
        .iter_errors(&valid_resp)
        .map(|e| e.to_string())
        .collect();
    assert!(
        errors.is_empty(),
        "合法 IngestSuccessResponse 应该校验通过，但出现错误: {:?}",
        errors
    );
}
