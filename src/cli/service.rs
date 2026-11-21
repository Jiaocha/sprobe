#[cfg(windows)]
use std::path::Path;
#[cfg(windows)]
use std::process::Command;

/// 注册 Windows 服务 SprobeAgent
#[cfg(windows)]
pub fn install_windows_service(config_path: Option<&Path>) -> Result<(), String> {
    let current_exe =
        std::env::current_exe().map_err(|e| format!("获取当前可执行文件路径失败: {e}"))?;
    let exe_path_str = current_exe.to_string_lossy();

    let bin_path_arg = if let Some(cfg) = config_path {
        format!("\"{}\" -c \"{}\"", exe_path_str, cfg.to_string_lossy())
    } else {
        format!("\"{}\"", exe_path_str)
    };

    println!("[INFO] 正在注册 Windows 服务: SprobeAgent");
    println!("[INFO] 启动命令: {bin_path_arg}");

    // 1. 创建服务
    let create_status = Command::new("sc.exe")
        .args([
            "create",
            "SprobeAgent",
            &format!("binPath= {bin_path_arg}"),
            "DisplayName= Sprobe Server Monitoring Agent",
            "start= delayed-auto",
        ])
        .status()
        .map_err(|e| format!("调用 sc.exe create 失败: {e}"))?;

    if !create_status.success() {
        return Err("注册服务失败，请确保使用管理员权限运行 (Run as Administrator)".to_string());
    }

    // 2. 设置服务描述
    let _ = Command::new("sc.exe")
        .args([
            "description",
            "SprobeAgent",
            "Lightweight high-performance server probe agent",
        ])
        .status();

    // 3. 配置失败自动恢复策略 (5s, 10s, 30s 重启，1 天重置计数)
    let failure_status = Command::new("sc.exe")
        .args([
            "failure",
            "SprobeAgent",
            "reset=",
            "86400",
            "actions=",
            "restart/5000/restart/10000/restart/30000",
        ])
        .status();

    if let Ok(s) = failure_status
        && s.success()
    {
        println!("[INFO] 已成功配置服务失败自动恢复策略。");
    }

    println!("[SUCCESS] Windows 服务 SprobeAgent 注册成功！");
    println!("[INFO] 可使用 'sc.exe start SprobeAgent' 或 'Start-Service SprobeAgent' 启动服务。");
    Ok(())
}

/// 卸载 Windows 服务 SprobeAgent
#[cfg(windows)]
pub fn uninstall_windows_service(purge: bool, data_dir: Option<&Path>) -> Result<(), String> {
    println!("[INFO] 正在停止并卸载 Windows 服务: SprobeAgent...");

    // 尝试停止服务
    let _ = Command::new("sc.exe")
        .args(["stop", "SprobeAgent"])
        .status();

    // 删除服务
    let delete_status = Command::new("sc.exe")
        .args(["delete", "SprobeAgent"])
        .status()
        .map_err(|e| format!("调用 sc.exe delete 失败: {e}"))?;

    if !delete_status.success() {
        return Err("卸载服务失败，请确保使用管理员权限运行 (Run as Administrator)".to_string());
    }

    println!("[SUCCESS] Windows 服务 SprobeAgent 已成功卸载。");

    // 若指定了 purge，清理数据目录
    if purge {
        if let Some(dir) = data_dir {
            if dir.exists() {
                println!("[INFO] 正在执行 --purge 清理数据目录: {}", dir.display());
                if let Err(e) = std::fs::remove_dir_all(dir) {
                    eprintln!("[WARN] 清理数据目录失败: {e}");
                } else {
                    println!("[SUCCESS] 数据目录已清理。");
                }
            }
        } else {
            let default_dir = Path::new(r"C:\ProgramData\Sprobe");
            if default_dir.exists() {
                println!(
                    "[INFO] 正在执行 --purge 清理默认数据目录: {}",
                    default_dir.display()
                );
                let _ = std::fs::remove_dir_all(default_dir);
            }
        }
    }

    Ok(())
}

/// 生成标准 Linux systemd 服务单元内容 (严格符合文档 7.5 节规范)
pub fn generate_systemd_unit(bin_path: Option<&str>, config_path: Option<&str>) -> String {
    let bin = bin_path.unwrap_or("/usr/local/bin/sprobe");
    let cfg = config_path.unwrap_or("/etc/sprobe/sprobe.toml");

    format!(
        r#"[Unit]
Description=Sprobe server agent
Documentation=https://example.com/sprobe/docs
After=network-online.target
Wants=network-online.target

[Service]
Type=exec
ExecStart={bin} --config {cfg}
ExecReload=/bin/kill -HUP $MAINPID
Restart=on-failure
RestartSec=5s
TimeoutStopSec=25s
User=sprobe
Group=sprobe
UMask=0027
NoNewPrivileges=true
PrivateTmp=true
PrivateDevices=false
ProtectSystem=strict
ProtectHome=true
ProtectKernelTunables=true
ProtectKernelModules=true
ProtectControlGroups=true
RestrictAddressFamilies=AF_UNIX AF_INET AF_INET6 AF_NETLINK
RestrictNamespaces=true
LockPersonality=true
MemoryDenyWriteExecute=true
SystemCallArchitectures=native
ReadWritePaths=/var/lib/sprobe /var/log/sprobe

[Install]
WantedBy=multi-user.target
"#
    )
}
