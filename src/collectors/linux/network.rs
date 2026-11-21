//! Linux /proc/net/dev 网络接口指标采集与解析

use crate::snapshot::{NetworkInterfaceItem, NetworkSection, Quality, SectionMeta, SectionStatus};
use chrono::Utc;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RawNetDevEntry {
    pub name: String,
    pub rx_bytes: u64,
    pub rx_packets: u64,
    pub rx_errors: u64,
    pub rx_dropped: u64,
    pub tx_bytes: u64,
    pub tx_packets: u64,
    pub tx_errors: u64,
    pub tx_dropped: u64,
}

/// 解析 /proc/net/dev
pub fn parse_proc_net_dev(content: &str) -> Vec<RawNetDevEntry> {
    let mut list = Vec::new();

    for line in content.lines() {
        if !line.contains(':') {
            continue;
        }

        let (name_part, stats_part) = match line.split_once(':') {
            Some(pair) => pair,
            None => continue,
        };

        let name = name_part.trim().to_string();
        let fields: Vec<u64> = stats_part
            .split_whitespace()
            .filter_map(|s| s.parse::<u64>().ok())
            .collect();

        if fields.len() < 16 {
            continue;
        }

        list.push(RawNetDevEntry {
            name,
            rx_bytes: fields[0],
            rx_packets: fields[1],
            rx_errors: fields[2],
            rx_dropped: fields[3],
            tx_bytes: fields[8],
            tx_packets: fields[9],
            tx_errors: fields[10],
            tx_dropped: fields[11],
        });
    }

    list
}

/// Linux 网络采集器
pub struct LinuxNetworkCollector {
    prev_entries: HashMap<String, (RawNetDevEntry, chrono::DateTime<Utc>)>,
}

impl Default for LinuxNetworkCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxNetworkCollector {
    pub fn new() -> Self {
        Self {
            prev_entries: HashMap::new(),
        }
    }

    pub fn collect_from_content(&mut self, content: &str) -> Result<NetworkSection, String> {
        self.collect_from_content_with_filter(content, &[], &[], false)
    }

    pub fn collect_from_content_with_filter(
        &mut self,
        content: &str,
        include_patterns: &[String],
        exclude_patterns: &[String],
        ignore_virtual: bool,
    ) -> Result<NetworkSection, String> {
        let entries = parse_proc_net_dev(content);
        let now = Utc::now();
        let mut interfaces = Vec::new();

        for curr in entries {
            let is_virtual = curr.name.starts_with("lo")
                || curr.name.starts_with("docker")
                || curr.name.starts_with("veth")
                || curr.name.starts_with("br-")
                || curr.name.starts_with("cni")
                || curr.name.starts_with("tailscale");

            // 应用网卡过滤规则
            if !crate::collectors::network::should_keep_interface(
                &curr.name,
                is_virtual,
                include_patterns,
                exclude_patterns,
                ignore_virtual,
            ) {
                continue;
            }

            let (rx_rate, tx_rate) =
                if let Some((prev, prev_time)) = self.prev_entries.get(&curr.name) {
                    let delta_secs = (now - *prev_time).num_milliseconds() as f64 / 1000.0;
                    if delta_secs > 0.05 {
                        let rx_delta = curr.rx_bytes.saturating_sub(prev.rx_bytes);
                        let tx_delta = curr.tx_bytes.saturating_sub(prev.tx_bytes);
                        (
                            Some(rx_delta as f64 / delta_secs),
                            Some(tx_delta as f64 / delta_secs),
                        )
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                };

            interfaces.push(NetworkInterfaceItem {
                name: curr.name.clone(),
                class: if is_virtual {
                    "virtual".to_string()
                } else {
                    "physical".to_string()
                },
                master: None,
                r#virtual: is_virtual,
                up: true,
                mtu: 1500,
                speed_bytes_per_second: None,
                receive_bytes_total: curr.rx_bytes,
                transmit_bytes_total: curr.tx_bytes,
                receive_packets_total: curr.rx_packets,
                transmit_packets_total: curr.tx_packets,
                receive_errors_total: curr.rx_errors,
                transmit_errors_total: curr.tx_errors,
                receive_dropped_total: curr.rx_dropped,
                transmit_dropped_total: curr.tx_dropped,
                receive_bytes_per_second: rx_rate,
                transmit_bytes_per_second: tx_rate,
                receive_utilization_ratio: None,
                transmit_utilization_ratio: None,
            });

            self.prev_entries.insert(curr.name.clone(), (curr, now));
        }

        interfaces.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(NetworkSection {
            meta: SectionMeta {
                status: SectionStatus::Ok,
                quality: Quality::Derived,
                sampled_at: now,
                age_seconds: 0.0,
                errors: Vec::new(),
            },
            truncated: false,
            interfaces,
            cycle_outbound_bytes: None,
            cycle_limit_bytes: None,
            cycle_outbound_ratio: None,
        })
    }

    #[cfg(target_os = "linux")]
    pub fn collect(&mut self) -> Result<NetworkSection, String> {
        let content = std::fs::read_to_string("/proc/net/dev")
            .map_err(|e| format!("读取 /proc/net/dev 失败: {e}"))?;
        self.collect_from_content(&content)
    }

    #[cfg(target_os = "linux")]
    pub fn collect_with_config(
        &mut self,
        config: &crate::config::NetworkConfig,
    ) -> Result<NetworkSection, String> {
        let content = std::fs::read_to_string("/proc/net/dev")
            .map_err(|e| format!("读取 /proc/net/dev 失败: {e}"))?;
        self.collect_from_content_with_filter(
            &content,
            &config.include_interfaces,
            &config.exclude_interfaces,
            config.ignore_virtual,
        )
    }
}
