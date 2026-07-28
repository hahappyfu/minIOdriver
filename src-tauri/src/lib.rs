use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::{Listener, Manager};
use tauri_plugin_autostart::ManagerExt;

mod credentials;
mod rclone;
mod tray;

// 配置数据结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub name: String,
    pub endpoint: String,
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bucket {
    pub name: String,
    pub drive: Option<String>,
    pub mounted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountInfo {
    pub bucket: String,
    pub drive: String,
    pub status: String,
}

// 应用状态
pub struct AppState {
    pub connections: Mutex<Vec<Connection>>,
    pub current_connection: Mutex<Option<Connection>>,
    pub mounts: Mutex<Vec<MountInfo>>,
    pub rclone_processes: Mutex<HashMap<String, rclone::RcloneProcess>>,
}

// ========== 连接管理命令 ==========

#[tauri::command]
async fn test_connection(endpoint: String, access_key: String, secret_key: String) -> Result<bool, String> {
    rclone::test_connection(&endpoint, &access_key, &secret_key)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn save_connection(state: tauri::State<'_, AppState>, connection: Connection) -> Result<(), String> {
    let mut connections = state.connections.lock().map_err(|e| e.to_string())?;

    // 更新或添加
    if let Some(pos) = connections.iter().position(|c| c.id == connection.id) {
        connections[pos] = connection;
    } else {
        connections.push(connection);
    }

    // 保存到文件
    credentials::save_connections(&connections).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn load_connections(state: tauri::State<'_, AppState>) -> Result<Vec<Connection>, String> {
    let connections = credentials::load_connections().map_err(|e| e.to_string())?;

    let mut state_connections = state.connections.lock().map_err(|e| e.to_string())?;
    *state_connections = connections.clone();

    Ok(connections)
}

#[tauri::command]
async fn delete_connection(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let mut connections = state.connections.lock().map_err(|e| e.to_string())?;
    connections.retain(|c| c.id != id);

    credentials::save_connections(&connections).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn set_current_connection(state: tauri::State<'_, AppState>, id: String) -> Result<(), String> {
    let connections = state.connections.lock().map_err(|e| e.to_string())?;
    let connection = connections.iter().find(|c| c.id == id).cloned();

    let mut current = state.current_connection.lock().map_err(|e| e.to_string())?;
    *current = connection;

    Ok(())
}

// ========== Bucket 管理命令 ==========

#[tauri::command]
async fn list_buckets(state: tauri::State<'_, AppState>) -> Result<Vec<Bucket>, String> {
    let (endpoint, access_key, secret_key) = {
        let current = state.current_connection.lock().map_err(|e| e.to_string())?;
        let connection = current.as_ref().ok_or("未选择连接")?;
        (
            connection.endpoint.clone(),
            connection.access_key.clone(),
            connection.secret_key.clone(),
        )
    };

    let bucket_names = rclone::list_buckets(&endpoint, &access_key, &secret_key)
        .await
        .map_err(|e| e.to_string())?;

    let mounts = state.mounts.lock().map_err(|e| e.to_string())?;

    let buckets: Vec<Bucket> = bucket_names
        .into_iter()
        .map(|name| {
            let mount = mounts.iter().find(|m| m.bucket == name);
            Bucket {
                name: name.clone(),
                drive: mount.map(|m| m.drive.clone()),
                mounted: mount.is_some(),
            }
        })
        .collect();

    Ok(buckets)
}

// ========== 挂载管理命令 ==========

#[tauri::command]
async fn mount_bucket(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    bucket: String,
    drive: String,
) -> Result<String, String> {
    let (endpoint, access_key, secret_key) = {
        let current = state.current_connection.lock().map_err(|e| e.to_string())?;
        let connection = current.as_ref().ok_or("未选择连接")?;
        (
            connection.endpoint.clone(),
            connection.access_key.clone(),
            connection.secret_key.clone(),
        )
    };

    // 检查是否已挂载
    {
        let mounts = state.mounts.lock().map_err(|e| e.to_string())?;
        if mounts.iter().any(|m| m.bucket == bucket) {
            return Err("该 Bucket 已挂载".into());
        }
        if mounts.iter().any(|m| m.drive == drive) {
            return Err("该盘符已被使用".into());
        }
    }

    // 启动 rclone 挂载
    let process = rclone::start_mount(
        &app,
        &endpoint,
        &access_key,
        &secret_key,
        &bucket,
        &drive,
    )
    .await
    .map_err(|e| e.to_string())?;

    // 更新状态
    let mut rclone_processes = state.rclone_processes.lock().map_err(|e| e.to_string())?;
    rclone_processes.insert(bucket.clone(), process);

    let mut mounts = state.mounts.lock().map_err(|e| e.to_string())?;
    mounts.push(MountInfo {
        bucket: bucket.clone(),
        drive: drive.clone(),
        status: "mounting".into(),
    });

    Ok(format!("正在挂载 {} 到 {}:", bucket, drive))
}

#[tauri::command]
async fn unmount_bucket(state: tauri::State<'_, AppState>, bucket: String) -> Result<(), String> {
    let mut rclone_processes = state.rclone_processes.lock().map_err(|e| e.to_string())?;

    if let Some(process) = rclone_processes.remove(&bucket) {
        process.kill().map_err(|e| e.to_string())?;
    }

    let mut mounts = state.mounts.lock().map_err(|e| e.to_string())?;
    mounts.retain(|m| m.bucket != bucket);

    Ok(())
}

#[tauri::command]
async fn unmount_all(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut rclone_processes = state.rclone_processes.lock().map_err(|e| e.to_string())?;

    // 杀死所有 rclone 进程
    for (_, process) in rclone_processes.drain() {
        process.kill().map_err(|e| e.to_string())?;
    }

    let mut mounts = state.mounts.lock().map_err(|e| e.to_string())?;
    mounts.clear();

    Ok(())
}

#[tauri::command]
async fn get_mount_status(state: tauri::State<'_, AppState>) -> Result<Vec<MountInfo>, String> {
    let mounts = state.mounts.lock().map_err(|e| e.to_string())?;
    Ok(mounts.clone())
}

// ========== 系统功能命令 ==========

#[tauri::command]
async fn check_winfsp() -> Result<bool, String> {
    Ok(rclone::check_winfsp())
}

#[tauri::command]
async fn set_autostart(app: tauri::AppHandle, enable: bool) -> Result<(), String> {
    if enable {
        app.autolaunch().enable().map_err(|e| e.to_string())?;
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_config(path: String) -> Result<(), String> {
    let connections = credentials::load_connections().map_err(|e| e.to_string())?;
    let json = serde_json::to_string_pretty(&connections).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn import_config(path: String) -> Result<(), String> {
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let connections: Vec<Connection> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    credentials::save_connections(&connections).map_err(|e| e.to_string())?;
    Ok(())
}

// Tauri 入口
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--autostart"]),
        ))
        .setup(|app| {
            // 初始化状态
            app.manage(AppState {
                connections: Mutex::new(Vec::new()),
                current_connection: Mutex::new(None),
                mounts: Mutex::new(Vec::new()),
                rclone_processes: Mutex::new(HashMap::new()),
            });

            // 创建系统托盘
            tray::create_tray(app)?;

            // 确保窗口显示（开发模式）
            if let Some(window) = app.get_webview_window("main") {
                window.show()?;
                window.set_focus()?;
            }

            // 点击窗口关闭按钮时隐藏而非退出（保持托盘运行）
            {
                let app_handle = app.handle().clone();
                app.listen("tauri://close-requested", move |event| {
                    if let Some(label) = event.payload().strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                        if label == "main" {
                            if let Some(window) = app_handle.get_webview_window("main") {
                                let _ = window.hide();
                            }
                        }
                    }
                });
            }

            // 应用退出时清理所有 rclone 进程
            {
                let app_handle = app.handle().clone();
                app.listen("tauri://exit", move |_| {
                    tray::cleanup_mounts(&app_handle);
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            test_connection,
            save_connection,
            load_connections,
            delete_connection,
            set_current_connection,
            list_buckets,
            mount_bucket,
            unmount_bucket,
            unmount_all,
            get_mount_status,
            check_winfsp,
            set_autostart,
            get_autostart,
            export_config,
            import_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
