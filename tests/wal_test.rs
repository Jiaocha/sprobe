use sprobe::snapshot::Snapshot;
use sprobe::wal::WalManager;
use tempfile::tempdir;

#[test]
fn test_wal_write_and_recover() {
    let dir = tempdir().expect("创建临时目录失败");
    let wal_dir = dir.path().join("wal");

    let fixture_str = include_str!("fixtures/golden_snapshot_v1_0.json");
    let mut snap1: Snapshot = serde_json::from_str(fixture_str).unwrap();
    snap1.sequence = 1;

    let mut snap2 = snap1.clone();
    snap2.sequence = 2;

    let mut snap3 = snap1.clone();
    snap3.sequence = 3;

    // 1. 写入 3 个快照
    {
        let mut wal = WalManager::open(&wal_dir, 10 * 1024 * 1024, "always").unwrap();
        wal.append_snapshot(&snap1).unwrap();
        wal.append_snapshot(&snap2).unwrap();
        wal.append_snapshot(&snap3).unwrap();
    }

    // 2. 模拟重启恢复
    {
        let wal = WalManager::open(&wal_dir, 10 * 1024 * 1024, "always").unwrap();
        let recovered = wal.recover_snapshots().unwrap();
        assert_eq!(recovered.len(), 3);
        assert_eq!(recovered[0].sequence, 1);
        assert_eq!(recovered[1].sequence, 2);
        assert_eq!(recovered[2].sequence, 3);
    }

    // 3. 推进 Ack 水位至 seq=2
    {
        let mut wal = WalManager::open(&wal_dir, 10 * 1024 * 1024, "always").unwrap();
        wal.update_ack_watermark(2).unwrap();
        // 再次恢复，seq <= 2 应该被过滤掉，只剩 seq=3
        let recovered = wal.recover_snapshots().unwrap();
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].sequence, 3);
    }
}
