use crate::config::PushConfig;
use crate::snapshot::{HealthStatus, Snapshot};
use chrono::{DateTime, Utc};
use flate2::Compression;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::time::Duration;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum PushError {
    #[error("网络请求错误: {0}")]
    Network(#[from] reqwest::Error),
    #[error("JSON 序列化失败: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Gzip 压缩失败: {0}")]
    Compression(#[from] std::io::Error),
    #[error("服务端拒绝 ({status}): {body}")]
    ServerError {
        status: reqwest::StatusCode,
        body: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestBatch {
    pub schema_version: String,
    pub batch_id: Uuid,
    pub agent_id: Uuid,
    pub instance_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub sequence_first: u64,
    pub sequence_last: u64,
    pub snapshot_count: usize,
    pub snapshots: Vec<Snapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestSuccessResponse {
    pub schema_version: String,
    pub status: String,
    pub batch_id: Uuid,
    pub accepted: usize,
    pub duplicates: usize,
    pub rejected: Vec<serde_json::Value>,
    pub server_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatRequest {
    pub schema_version: String,
    pub agent_id: Uuid,
    pub instance_id: Uuid,
    pub sent_at: DateTime<Utc>,
    pub uptime_seconds: f64,
    pub ready: bool,
    pub warmup: bool,
    pub health_status: HealthStatus,
    pub health_status_code: u8,
    pub last_sequence: u64,
    pub wal_snapshots: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub status: String,
    pub server_time: Option<String>,
}

pub struct PushClient {
    endpoint: String,
    heartbeat_endpoint: String,
    token: Option<String>,
    gzip: bool,
    client: reqwest::blocking::Client,
}

impl PushClient {
    pub fn new(config: &PushConfig, token: Option<String>) -> Result<Self, PushError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs_f64(config.request_timeout_seconds))
            .connect_timeout(Duration::from_secs_f64(config.connect_timeout_seconds))
            .build()?;

        let heartbeat_endpoint = if !config.heartbeat_endpoint.is_empty() {
            config.heartbeat_endpoint.clone()
        } else if let Some(base) = config.endpoint.strip_suffix("/ingest") {
            format!("{base}/heartbeat")
        } else {
            config
                .endpoint
                .replace("/api/v1/ingest", "/api/v1/heartbeat")
        };

        Ok(Self {
            endpoint: config.endpoint.clone(),
            heartbeat_endpoint,
            token,
            gzip: config.gzip,
            client,
        })
    }

    pub fn send_batch(&self, batch: &IngestBatch) -> Result<IngestSuccessResponse, PushError> {
        let json_bytes = serde_json::to_vec(batch)?;

        let mut req = self.client.post(&self.endpoint);
        req = req.header("Content-Type", "application/json");

        if let Some(ref t) = self.token {
            req = req.header("Authorization", format!("Bearer {t}"));
        }

        let body_bytes = if self.gzip {
            req = req.header("Content-Encoding", "gzip");
            let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(&json_bytes)?;
            encoder.finish()?
        } else {
            json_bytes
        };

        let res = req.body(body_bytes).send()?;
        let status = res.status();

        if status.is_success() {
            let resp_obj: IngestSuccessResponse = res.json()?;
            Ok(resp_obj)
        } else {
            let err_text = res.text().unwrap_or_default();
            Err(PushError::ServerError {
                status,
                body: err_text,
            })
        }
    }

    pub fn push_snapshots(
        &self,
        agent_id: Uuid,
        instance_id: Uuid,
        batch_id: Uuid,
        snapshots: Vec<Snapshot>,
    ) -> Result<IngestSuccessResponse, PushError> {
        let count = snapshots.len();
        let first = snapshots.first().map(|s| s.sequence).unwrap_or(0);
        let last = snapshots.last().map(|s| s.sequence).unwrap_or(0);

        let batch = IngestBatch {
            schema_version: "1.0".to_string(),
            batch_id,
            agent_id,
            instance_id,
            created_at: Utc::now(),
            sequence_first: first,
            sequence_last: last,
            snapshot_count: count,
            snapshots,
        };
        self.send_batch(&batch)
    }

    pub fn send_heartbeat(&self, req: &HeartbeatRequest) -> Result<HeartbeatResponse, PushError> {
        let mut request = self.client.post(&self.heartbeat_endpoint);
        request = request.header("Content-Type", "application/json");

        if let Some(ref t) = self.token {
            request = request.header("Authorization", format!("Bearer {t}"));
        }

        let res = request.json(req).send()?;
        let status = res.status();

        if status.is_success() {
            let resp_obj: HeartbeatResponse = res.json()?;
            Ok(resp_obj)
        } else {
            let err_text = res.text().unwrap_or_default();
            Err(PushError::ServerError {
                status,
                body: err_text,
            })
        }
    }
}
