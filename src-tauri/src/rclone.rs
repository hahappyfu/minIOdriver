use anyhow::{Context, Result};
use std::process::Command as StdCommand;
use tauri::Emitter;
use tauri_plugin_shell::{process::CommandChild, process::CommandEvent, ShellExt};

pub struct RcloneProcess {
    child: CommandChild,
    config_path: std::path::PathBuf,
}

impl RcloneProcess {
    pub fn kill(self) -> Result<()> {
        // 先清理配置文件
        let _ = std::fs::remove_file(&self.config_path);

        self.child.kill().context("Failed to kill rclone process")?;
        Ok(())
    }
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

/// 获取 rclone 可执行文件路径
fn get_rclone_path() -> String {
    // 开发模式下，使用项目目录中的 rclone
    // 生产模式下，Tauri 会自动处理 sidecar 路径
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let path = std::path::Path::new(manifest_dir)
        .join("binaries")
        .join("rclone-aarch64-apple-darwin");
    path.to_string_lossy().to_string()
}

/// 测试 MinIO 连接
pub async fn test_connection(endpoint: &str, access_key: &str, secret_key: &str) -> Result<bool> {
    // 生成临时配置文件供 rclone 使用
    let config_path = generate_rclone_config(endpoint, access_key, secret_key)?;
    let config_for_cleanup = config_path.clone();
    let config_str = config_path.to_string_lossy().to_string();

    // 克隆为 owned 值以满足 spawn_blocking 的 'static 要求
    let ep = endpoint.to_string();
    let ak = access_key.to_string();
    let sk = secret_key.to_string();
    let rclone_path = get_rclone_path();

    // 使用 10 秒超时防止卡死
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::task::spawn_blocking(move || {
            StdCommand::new(&rclone_path)
                .args([
                    "lsd",
                    "minio:",
                    "--config",
                    &config_str,
                    "--s3-provider",
                    "Minio",
                    "--s3-endpoint",
                    &ep,
                    "--s3-access-key-id",
                    &ak,
                    "--s3-secret-access-key",
                    &sk,
                    "--max-depth",
                    "0",
                ])
                .output()
        }),
    )
    .await;

    // 无论成功失败都清理临时配置文件
    let _ = std::fs::remove_file(&config_for_cleanup);

    match result {
        Ok(Ok(Ok(output))) => Ok(output.status.success()),
        Ok(Ok(Err(_))) => Ok(false), // rclone 执行失败（命令不存在等）
        Ok(Err(_)) => Ok(false),     // spawn_blocking 内部 panic
        Err(_) => Ok(false),         // 超时
    }
}

/// 列出所有 Bucket
pub async fn list_buckets(endpoint: &str, access_key: &str, secret_key: &str) -> Result<Vec<String>> {
    // 生成临时配置文件
    let config_path = generate_rclone_config(endpoint, access_key, secret_key)?;
    let config_for_cleanup = config_path.clone();
    let config_str = config_path.to_string_lossy().to_string();

    let ep = endpoint.to_string();
    let ak = access_key.to_string();
    let sk = secret_key.to_string();
    let rclone_path = get_rclone_path();

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        tokio::task::spawn_blocking(move || {
            StdCommand::new(&rclone_path)
                .args([
                    "lsd",
                    "minio:",
                    "--config",
                    &config_str,
                    "--s3-provider",
                    "Minio",
                    "--s3-endpoint",
                    &ep,
                    "--s3-access-key-id",
                    &ak,
                    "--s3-secret-access-key",
                    &sk,
                ])
                .output()
        }),
    )
    .await;

    // 清理临时配置文件
    let _ = std::fs::remove_file(&config_for_cleanup);

    let output = match result {
        Ok(Ok(Ok(output))) => output,
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
    // 生成临时 rclone 配置
    let config_path = generate_rclone_config(endpoint, access_key, secret_key)?;

    let sidecar = app
        .shell()
        .sidecar("rclone")
        .context("Failed to get rclone sidecar")?;

    let config_path_str = config_path.to_string_lossy().to_string();
    let bucket_clone = bucket.to_string();

    let (mut rx, child) = sidecar
        .args([
            "mount",
            &format!("minio:{}", bucket),
            &format!("{}:", drive),
            "--config", &config_path_str,
            "--vfs-cache-mode", "full",
            "--volname", &format!("MinIO-{}", bucket),
            "--vfs-cache-max-size", "10G",
            "--vfs-cache-max-age", "1h",
            "--log-level", "INFO",
        ])
        .spawn()
        .context("Failed to spawn rclone")?;

    // 创建 oneshot channel 用于挂载成功确认
    let (tx, rx_mount) = tokio::sync::oneshot::channel::<bool>();

    // 后台监听输出
    let app_clone = app.clone();
    tauri::async_runtime::spawn(async move {
        let mut mounted = false;
        let mut tx = Some(tx);

        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    let line = String::from_utf8_lossy(&line);
                    println!("rclone stdout: {}", line);

                    // 检测挂载成功（rclone 通常输出 "The service rclone has started" 或类似信息）
                    if !mounted && (line.contains("The service rclone has started") ||
                                   line.contains("Serving remote") ||
                                   line.contains("Mounting") && line.contains("on"))
                    {
                        mounted = true;
                        let _ = app_clone.emit("rclone:mounted", &bucket_clone);

                        // 通知挂载成功
                        if let Some(tx) = tx.take() {
                            let _ = tx.send(true);
                        }
                    }
                }
                CommandEvent::Stderr(line) => {
                    let line = String::from_utf8_lossy(&line);
                    eprintln!("rclone stderr: {}", line);

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
                    let _ = app_clone.emit("rclone:terminated", &bucket_clone);

                    // 如果还未挂载就终止了，通知失败
                    if !mounted {
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
                config_path,
            })
        }
        Ok(Ok(false)) => {
            // 挂载失败，进程可能还在运行，需要清理
            let _ = child.kill();
            let _ = std::fs::remove_file(&config_path);
            anyhow::bail!("rclone mount failed")
        }
        Ok(Err(_)) => {
            // channel 被关闭
            let _ = child.kill();
            let _ = std::fs::remove_file(&config_path);
            anyhow::bail!("rclone mount channel closed unexpectedly")
        }
        Err(_) => {
            // 超时，认为挂载成功（某些情况下 rclone 不输出成功消息）
            println!("rclone mount timeout, assuming success for {}", bucket);
            Ok(RcloneProcess {
                child,
                config_path,
            })
        }
    }
}

/// 生成临时 rclone 配置文件
fn generate_rclone_config(endpoint: &str, access_key: &str, secret_key: &str) -> Result<std::path::PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("minio-drive");

    std::fs::create_dir_all(&config_dir)?;

    let config_path = config_dir.join("rclone.conf");

    let config_content = format!(
        r#"[minio]
type = s3
provider = Minio
env_auth = false
access_key_id = {access_key}
secret_access_key = {secret_key}
region =
endpoint = {endpoint}
acl = private
"#
    );

    std::fs::write(&config_path, config_content)?;

    Ok(config_path)
}
