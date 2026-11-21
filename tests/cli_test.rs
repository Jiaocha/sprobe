use sprobe::cli::{generate_diagnose_package, generate_systemd_unit};
use sprobe::config::SprobeConfig;
use tempfile::tempdir;

#[test]
fn test_generate_systemd_unit() {
    let unit = generate_systemd_unit(Some("/usr/bin/sprobe"), Some("/etc/sprobe/custom.toml"));

    assert!(unit.contains("ExecStart=/usr/bin/sprobe --config /etc/sprobe/custom.toml"));
    assert!(unit.contains("ProtectSystem=strict"));
    assert!(unit.contains("Restart=on-failure"));
    assert!(unit.contains("TimeoutStopSec=25s"));
    assert!(unit.contains("UMask=0027"));
    assert!(unit.contains("RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6 AF_NETLINK"));
}

#[test]
fn test_diagnose_package_redacted() {
    let dir = tempdir().unwrap();
    let config = SprobeConfig::default();

    let diag = generate_diagnose_package(&config, dir.path()).expect("生成诊断包失败");

    assert_eq!(diag.agent_version, env!("CARGO_PKG_VERSION"));
    assert_eq!(diag.wal.dir, dir.path().display().to_string());
    assert_eq!(diag.wal.pending_count, 0);

    // 检查脱敏配置中不含密钥
    let json_str = serde_json::to_string(&diag).unwrap();
    assert!(!json_str.contains("bearer "));
    assert!(!json_str.contains("secret"));
    assert!(json_str.contains("\"capabilities\""));
}
