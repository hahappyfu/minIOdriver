use anyhow::{Context, Result};
use std::io::Write;
use tauri::Emitter;
use tauri::Manager;
use tauri_plugin_shell::{process::CommandChild, process::CommandEvent, ShellExt};

fn get_log_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("minio-drive")
        .join("logs")
}

fn init_log_file() -> std::fs::File {
    let log_dir = get_log_path();
    std::fs::create_dir_all(&log_dir).ok();
    let log_path = log_dir.join(format!("rclone-{}.log",
        chrono::Utc::now().format("%Y%m%d")));
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
        .unwrap()
}

pub struct RcloneProcess {
    child: CommandChild,
}

impl RcloneProcess {
    pub fn kill(self) -> Result<()> {
        self.child.kill().context("Failed to kill rclone process")?;
        Ok(())
    }
}

/// 用于自动重连的挂载配置
#[derive(Clone)]
pub struct MountConfig {
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    pub drive: String,
}

/// 检查 WinFsp 是否安装
pub fn check_winfsp() -> bool {
    // 检查 Windows 注册表或 DLL
    #[cfg(target_os = "windows")]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        if let Ok(key) = hklm.open_subkey("SOFTWARE\\WOW6432Node\\WinFsp") {
            if let Ok(_install_path) = key.get_value::<String, _>("InstallDir") {
                return true;
            }
        }

        // 也检查 64 位注册表
        if let Ok(key) = hklm.open_subkey("SOFTWARE\\WinFsp") {
            if let Ok(_install_path) = key.get_value::<String, _>("InstallDir") {
                return true;
            }
        }

        false
    }

    #[cfg(not(target_os = "windows"))]
    {
        // 非 Windows 平台，假设已安装
        true
    }
}

/// 测试 MinIO 连接
pub async fn test_connection(
    app: &tauri::AppHandle,
    endpoint: &str,
    access_key: &str,
    secret_key: &str,
) -> Result<bool> {
    // 克隆为 owned 值以满足异步命令的 'static 要求
    let ep = endpoint.to_string();
    let ak = access_key.to_string();
    let sk = secret_key.to_string();

    // 使用 sidecar 获取 rclone 可执行文件
    let sidecar = app
        .shell()
        .sidecar("rclone")
        .context("Failed to get rclone sidecar")?;

    // 使用内联参数，不依赖配置文件
    let remote = format!(
        ":s3,provider=Minio,endpoint={ep},access_key_id={ak},secret_access_key={sk}:"
    );

    // 使用 10 秒超时防止卡死
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        sidecar
            .args([
                "lsd",
                &remote,
                "--max-depth",
                "0",
            ])
            .output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => Ok(output.status.success()),
        Ok(Err(_)) => Ok(false), // sidecar 执行失败
        Err(_) => Ok(false),     // 超时
    }
}

/// 列出所有 Bucket
pub async fn list_buckets(
    app: &tauri::AppHandle,
    endpoint: &str,
    access_key: &str,
    secret_key: &str,
) -> Result<Vec<String>> {
    let ep = endpoint.to_string();
    let ak = access_key.to_string();
    let sk = secret_key.to_string();

    // 使用 sidecar 获取 rclone 可执行文件
    let sidecar = app
        .shell()
        .sidecar("rclone")
        .context("Failed to get rclone sidecar")?;

    // 使用内联参数，不依赖配置文件
    let remote = format!(
        ":s3,provider=Minio,endpoint={ep},access_key_id={ak},secret_access_key={sk}:"
    );

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        sidecar
            .args([
                "lsd",
                &remote,
            ])
            .output(),
    )
    .await;

    let output = match result {
        Ok(Ok(output)) => output,
        _ => anyhow::bail!("连接超时或 rclone 执行失败"),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Failed to list buckets: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let buckets: Vec<String> = stdout
        .lines()
        .filter_map(|line| {
            // rclone lsd 输出格式: "          -1 2024-01-01 00:00:00        -1 bucket-name"
            let parts: Vec<&str> = line.split_whitespace().collect();
            parts.last().map(|s| s.to_string())
        })
        .collect();

    Ok(buckets)
}

/// 启动 rclone 挂载
pub async fn start_mount(
    app: &tauri::AppHandle,
    endpoint: &str,
    access_key: &str,
    secret_key: &str,
    bucket: &str,
    drive: &str,
) -> Result<RcloneProcess> {
    // 读取缓存配置
    let cache_config = crate::load_cache_config().unwrap_or_default();

    let sidecar = app
        .shell()
        .sidecar("rclone")
        .context("Failed to get rclone sidecar")?;

    let bucket_clone = bucket.to_string();
    let cache_max_size = format!("{}G", cache_config.max_size_gb);
    let cache_max_age = format!("{}h", cache_config.max_age_hours);

    // 使用内联参数，不依赖配置文件
    let remote = format!(
        ":s3,provider=Minio,endpoint={endpoint},access_key_id={access_key},secret_access_key={secret_key},bucket_region=:"
    );

    let (mut rx, child) = sidecar
        .args([
            "mount",
            &remote,
            &format!("{}:", drive),
            "--vfs-cache-mode", "full",
            "--volname", &format!("MinIO-{}", bucket),
            "--vfs-cache-max-size", &cache_max_size,
            "--vfs-cache-max-age", &cache_max_age,
            "--log-level", "INFO",
        ])
        .spawn()
        .context("Failed to spawn rclone")?;

    // 创建 oneshot channel 用于挂载成功确认
    let (tx, rx_mount) = tokio::sync::oneshot::channel::<bool>();

    // 初始化日志文件
    let mut log_file = init_log_file();

    // 后台监听输出
    let app_clone = app.clone();
    let mount_config = MountConfig {
        endpoint: endpoint.to_string(),
        access_key: access_key.to_string(),
        secret_key: secret_key.to_string(),
        bucket: bucket.to_string(),
        drive: drive.to_string(),
    };

    tauri::async_runtime::spawn(async move {
        let mut mounted = false;
        let mut tx = Some(tx);
        let mut retry_count = 0;
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY: u64 = 3;

        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let line = String::from_utf8_lossy(&line);
                    println!("rclone stdout: {}", line);

                    // 写入日志文件
                    if let Err(e) = writeln!(log_file, "rclone stdout: {}", line) {
                        eprintln!("Failed to write log: {}", e);
                    }

                    // 检测挂载成功（rclone 通常输出 "The service rclone has started" 或类似信息）
                    if !mounted && (line.contains("The service rclone has started") ||
                                   line.contains("Serving remote") ||
                                   line.contains("Mounting") && line.contains("on"))
                    {
                        mounted = true;
                        let _ = app_clone.emit("rclone:mounted", &bucket_clone);

                        // 更新挂载状态为已连接
                        if let Some(app_state) = app_clone.try_state::<crate::AppState>() {
                            if let Ok(mut mounts) = app_state.mounts.lock() {
                                if let Some(mount) = mounts.iter_mut().find(|m| m.bucket == bucket_clone) {
                                    mount.status = "connected".to_string();
                                }
                            }
                        }

                        // 通知挂载状态变化
                        let _ = app_clone.emit("rclone:status_changed", &bucket_clone);

                        // 通知挂载成功
                        if let Some(tx) = tx.take() {
                            let _ = tx.send(true);
                        }
                    }
                }
                CommandEvent::Stderr(line) => {
                    let line = String::from_utf8_lossy(&line);
                    eprintln!("rclone stderr: {}", line);

                    // 写入日志文件
                    if let Err(e) = writeln!(log_file, "rclone stderr: {}", line) {
                        eprintln!("Failed to write log: {}", e);
                    }

                    // 检测错误
                    if line.contains("Fatal error") || line.contains("Failed to mount") {
                        let _ = app_clone.emit("rclone:error", &line);
                        if let Some(tx) = tx.take() {
                            let _ = tx.send(false);
                        }
                    }
                }
                CommandEvent::Terminated(status) => {
                    println!("rclone terminated: {:?}", status);

                    // 写入日志文件
                    if let Err(e) = writeln!(log_file, "rclone terminated: {:?}", status) {
                        eprintln!("Failed to write log: {}", e);
                    }

                    // 如果已经挂载成功后终止，尝试自动重连
                    if mounted {
                        // 更新状态为断开连接
                        if let Some(app_state) = app_clone.try_state::<crate::AppState>() {
                            if let Ok(mut mounts) = app_state.mounts.lock() {
                                if let Some(mount) = mounts.iter_mut().find(|m| m.bucket == bucket_clone) {
                                    mount.status = "disconnected".to_string();
                                }
                            }
                        }

                        // 通知状态变化
                        let _ = app_clone.emit("rclone:status_changed", &bucket_clone);
                        let _ = app_clone.emit("rclone:terminated", &bucket_clone);

                        // 尝试自动重连
                        while retry_count < MAX_RETRIES {
                            retry_count += 1;
                            println!("尝试自动重连 {} (第 {} 次)...", bucket_clone, retry_count);

                            // 更新状态为重连中
                            if let Some(app_state) = app_clone.try_state::<crate::AppState>() {
                                if let Ok(mut mounts) = app_state.mounts.lock() {
                                    if let Some(mount) = mounts.iter_mut().find(|m| m.bucket == bucket_clone) {
                                        mount.status = "reconnecting".to_string();
                                    }
                                }
                            }
                            let _ = app_clone.emit("rclone:status_changed", &bucket_clone);

                            // 延迟后重试
                            tokio::time::sleep(tokio::time::Duration::from_secs(RETRY_DELAY)).await;

                            // 尝试重新挂载
                            let sidecar = match app_clone.shell().sidecar("rclone") {
                                Ok(s) => s,
                                Err(e) => {
                                    eprintln!("Failed to get rclone sidecar: {}", e);
                                    continue;
                                }
                            };

                            // 使用内联参数，不依赖配置文件
                            let remote = format!(
                                ":s3,provider=Minio,endpoint={},access_key_id={},secret_access_key={},bucket_region=:",
                                mount_config.endpoint, mount_config.access_key, mount_config.secret_key
                            );

                            let cache_config = crate::load_cache_config().unwrap_or_default();
                            let cache_max_size = format!("{}G", cache_config.max_size_gb);
                            let cache_max_age = format!("{}h", cache_config.max_age_hours);

                            match sidecar.args([
                                "mount",
                                &remote,
                                &format!("{}:", mount_config.drive),
                                "--vfs-cache-mode", "full",
                                "--volname", &format!("MinIO-{}", mount_config.bucket),
                                "--vfs-cache-max-size", &cache_max_size,
                                "--vfs-cache-max-age", &cache_max_age,
                                "--log-level", "INFO",
                            ]).spawn() {
                                Ok((new_rx, new_child)) => {
                                    // 重连成功，更新状态为 connecting
                                    if let Some(app_state) = app_clone.try_state::<crate::AppState>() {
                                        if let Ok(mut mounts) = app_state.mounts.lock() {
                                            if let Some(mount) = mounts.iter_mut().find(|m| m.bucket == bucket_clone) {
                                                mount.status = "connecting".to_string();
                                            }
                                        }
                                    }
                                    let _ = app_clone.emit("rclone:status_changed", &bucket_clone);

                                    // 更新进程引用
                                    if let Some(app_state) = app_clone.try_state::<crate::AppState>() {
                                        if let Ok(mut processes) = app_state.rclone_processes.lock() {
                                            // 创建新的 RcloneProcess 并插入
                                            let new_process = RcloneProcess {
                                                child: new_child,
                                            };
                                            processes.insert(bucket_clone.clone(), new_process);
                                        }
                                    }

                                    // 继续监听新进程
                                    rx = new_rx;
                                    retry_count = 0; // 重置重试计数
                                    break; // 退出重连循环，继续监听
                                }
                                Err(e) => {
                                    eprintln!("重连失败: {}", e);
                                }
                            }
                        }

                        // 如果所有重试都失败
                        if retry_count >= MAX_RETRIES {
                            println!("自动重连失败，已达到最大重试次数");

                            // 更新状态为错误
                            if let Some(app_state) = app_clone.try_state::<crate::AppState>() {
                                if let Ok(mut mounts) = app_state.mounts.lock() {
                                    if let Some(mount) = mounts.iter_mut().find(|m| m.bucket == bucket_clone) {
                                        mount.status = "error".to_string();
                                    }
                                }
                            }
                            let _ = app_clone.emit("rclone:status_changed", &bucket_clone);
                            let _ = app_clone.emit("rclone:reconnect_failed", &bucket_clone);
                        }
                    } else {
                        // 挂载失败就终止了
                        let _ = app_clone.emit("rclone:terminated", &bucket_clone);
                        if let Some(tx) = tx.take() {
                            let _ = tx.send(false);
                        }
                    }
                    break;
                }
                _ => {}
            }
        }
    });

    // 等待挂载成功或超时（10秒）
    let mount_result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        rx_mount
    ).await;

    match mount_result {
        Ok(Ok(true)) => {
            println!("rclone mount succeeded for {}", bucket);
            Ok(RcloneProcess {
                child,
            })
        }
        Ok(Ok(false)) => {
            // 挂载失败，进程可能还在运行，需要清理
            let _ = child.kill();
            anyhow::bail!("rclone mount failed")
        }
        Ok(Err(_)) => {
            // channel 被关闭
            let _ = child.kill();
            anyhow::bail!("rclone mount channel closed unexpectedly")
        }
        Err(_) => {
            // 超时，认为挂载成功（某些情况下 rclone 不输出成功消息）
            println!("rclone mount timeout, assuming success for {}", bucket);
            Ok(RcloneProcess {
                child,
            })
        }
    }
}
