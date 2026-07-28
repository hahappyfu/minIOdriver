use crate::AppState;
use std::sync::OnceLock;
use tauri::{
    menu::{MenuBuilder, MenuItem, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder},
    tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, Wry,
};
use tauri_plugin_autostart::ManagerExt;

/// 全局存储托盘句柄，供菜单事件处理中动态重建菜单
static TRAY_ICON: OnceLock<std::sync::Mutex<Option<TrayIcon<Wry>>>> = OnceLock::new();

fn get_tray() -> Option<std::sync::MutexGuard<'static, Option<TrayIcon<Wry>>>> {
    TRAY_ICON.get().map(|m| m.lock().unwrap())
}

/// 创建系统托盘（菜单项根据实际状态动态构建）
pub fn create_tray(app: &tauri::App) -> tauri::Result<()> {
    let menu = build_menu(app)?;
    let mount_count = {
        let state = app.state::<AppState>();
        state.mounts.lock().map(|m| m.len()).unwrap_or(0)
    };

    let tray = TrayIconBuilder::with_id("main-tray")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip(&format!("MinIO Drive - 已挂载 {} 个 Bucket", mount_count))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| handle_menu_event(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;

    // 存储 TrayIcon 到全局静态，供后续菜单重建使用
    let _ = TRAY_ICON.set(std::sync::Mutex::new(Some(tray)));

    Ok(())
}

/// 根据当前应用状态构建托盘菜单（接受 &App，用于初始创建）
fn build_menu(app: &tauri::App) -> tauri::Result<tauri::menu::Menu<Wry>> {
    let state = app.state::<AppState>();

    let (mount_count, mounted_buckets, has_mounts) = {
        let mounts = state.mounts.lock().unwrap();
        let count = mounts.len();
        let buckets: Vec<String> = mounts.iter().map(|m| m.bucket.clone()).collect();
        (count, buckets, count > 0)
    };

    let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);

    let show_i = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let status_text = if mount_count > 0 {
        format!("状态: {} 个 Bucket 已挂载", mount_count)
    } else {
        "状态: 未连接".to_string()
    };
    let status_i = MenuItem::with_id(app, "status", &status_text, false, None::<&str>)?;
    let mount_i = MenuItem::with_id(app, "mount", "快速挂载", true, None::<&str>)?;
    let unmount_i = MenuItem::with_id(app, "unmount", "卸载全部", has_mounts, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let autostart_label = if autostart_enabled {
        "开机自启  \u{2713}"
    } else {
        "开机自启"
    };
    let autostart_i = MenuItem::with_id(app, "autostart", autostart_label, true, None::<&str>)?;

    // "已挂载 Bucket" 子菜单（仅当有挂载时构建）
    let mounted_menu = if has_mounts {
        let mut builder = SubmenuBuilder::new(app, "已挂载 Bucket");
        for bucket in &mounted_buckets {
            let item = MenuItemBuilder::new(format!("{}  \u{2716}", bucket))
                .id(format!("unmount:{}", bucket))
                .enabled(true)
                .build(app)?;
            builder = builder.item(&item);
        }
        Some(builder.build()?)
    } else {
        None
    };

    let mut builder = MenuBuilder::new(app)
        .item(&show_i)
        .item(&status_i)
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&mount_i)
        .item(&unmount_i)
        .item(&autostart_i)
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&quit_i);

    if let Some(ref submenu) = mounted_menu {
        builder = builder.item(submenu);
    }

    builder.build().map_err(|e| e.into())
}

/// 处理托盘菜单点击事件
fn handle_menu_event(app: &tauri::AppHandle, id: &str) {
    match id {
        "show" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "mount" => {
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                handle_quick_mount(&app_clone).await;
            });
        }
        "unmount" => {
            cleanup_mounts(app);
            let _ = app.emit("rclone:unmounted_all", ());
            let _ = rebuild_menu(app);
        }
        "autostart" => {
            let is_enabled = app.autolaunch().is_enabled().unwrap_or(false);
            if is_enabled {
                let _ = app.autolaunch().disable();
            } else {
                let _ = app.autolaunch().enable();
            }
            let _ = rebuild_menu(app);
        }
        "quit" => {
            cleanup_mounts(app);
            app.exit(0);
        }
        s if s.starts_with("unmount:") => {
            let bucket = s["unmount:".len()..].to_string();
            let app_clone = app.clone();
            tauri::async_runtime::spawn(async move {
                let state = app_clone.state::<AppState>();
                if let Some(process) = state
                    .rclone_processes
                    .lock()
                    .unwrap()
                    .remove(&bucket)
                {
                    let _ = process.kill();
                }
                state
                    .mounts
                    .lock()
                    .unwrap()
                    .retain(|m| m.bucket != bucket);
                let _ = app_clone.emit("rclone:unmounted", &bucket);
                let _ = rebuild_menu(&app_clone);
            });
        }
        _ => {}
    }
}

/// "快速挂载"：将所有已配置但尚未挂载的 Bucket 挂载到空闲盘符
async fn handle_quick_mount(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();

    let (endpoint, access_key, secret_key, mounted_buckets) = {
        let current = state.current_connection.lock().unwrap();
        let connection = match current.as_ref() {
            Some(c) => c.clone(),
            None => {
                let _ = app.emit("rclone:error", "未配置连接，请先在应用中添加连接");
                return;
            }
        };
        let mounts = state.mounts.lock().unwrap();
        let mounted: Vec<String> = mounts.iter().map(|m| m.bucket.clone()).collect();
        (
            connection.endpoint,
            connection.access_key,
            connection.secret_key,
            mounted,
        )
    };

    let bucket_names = match crate::rclone::list_buckets(app, &endpoint, &access_key, &secret_key).await
    {
        Ok(buckets) => buckets,
        Err(e) => {
            let _ = app.emit("rclone:error", format!("获取 Bucket 列表失败: {}", e));
            return;
        }
    };

    let unmounted: Vec<String> = bucket_names
        .into_iter()
        .filter(|b| !mounted_buckets.contains(b))
        .collect();

    if unmounted.is_empty() {
        let _ = app.emit("rclone:error", "没有可挂载的 Bucket（全部已挂载）");
        return;
    }

    // 为每个 Bucket 分配盘符（从 D 开始找第一个未使用的）
    let mut next_drive = 'D' as u8;
    {
        let mounts = state.mounts.lock().unwrap();
        let used: Vec<char> = mounts.iter().filter_map(|m| m.drive.chars().next()).collect();
        while (next_drive as char) <= 'Z' {
            if !used.contains(&(next_drive as char)) {
                break;
            }
            next_drive += 1;
        }
    }

    for bucket in unmounted {
        if next_drive > 'Z' as u8 {
            let _ = app.emit("rclone:error", format!("盘符已用尽，无法挂载 {}", bucket));
            break;
        }
        let drive = (next_drive as char).to_string();
        next_drive += 1;

        let app_clone = app.clone();
        let ep = endpoint.clone();
        let ak = access_key.clone();
        let sk = secret_key.clone();
        let bk = bucket.clone();
        let dr = drive.clone();

        tauri::async_runtime::spawn(async move {
            match crate::rclone::start_mount(&app_clone, &ep, &ak, &sk, &bk, &dr).await {
                Ok(process) => {
                    let state = app_clone.state::<AppState>();
                    state
                        .rclone_processes
                        .lock()
                        .unwrap()
                        .insert(bk.clone(), process);
                    state.mounts.lock().unwrap().push(crate::MountInfo {
                        bucket: bk.clone(),
                        drive: dr,
                        status: "mounted".into(),
                    });
                    let _ = app_clone.emit("rclone:mounted", &bk);
                    let _ = rebuild_menu(&app_clone);
                }
                Err(e) => {
                    let _ = app_clone.emit("rclone:error", format!("挂载 {} 失败: {}", bk, e));
                }
            }
        });
    }
}

/// 清理所有 rclone 进程和挂载记录（用于退出/窗口关闭）
pub fn cleanup_mounts(app: &tauri::AppHandle) {
    let state = app.state::<AppState>();
    let mut processes = state.rclone_processes.lock().unwrap();
    for (_, process) in processes.drain() {
        let _ = process.kill();
    }
    state.mounts.lock().unwrap().clear();
}

/// 根据最新状态重建托盘菜单（通过全局存储的 TrayIcon）
fn rebuild_menu(app: &tauri::AppHandle) -> tauri::Result<()> {
    let new_menu = build_menu_from_handle(app)?;
    let mount_count = {
        let state = app.state::<AppState>();
        state.mounts.lock().map(|m| m.len()).unwrap_or(0)
    };

    if let Some(guard) = get_tray() {
        if let Some(ref tray) = *guard {
            let _ = tray.set_menu(Some(new_menu));
            let _ = tray.set_tooltip(Some(&format!(
                "MinIO Drive - 已挂载 {} 个 Bucket",
                mount_count
            )));
        }
    }

    Ok(())
}

/// 从 &AppHandle 构建菜单（用于动态重建，AppHandle 实现了 Manager）
fn build_menu_from_handle(app: &tauri::AppHandle) -> tauri::Result<tauri::menu::Menu<Wry>> {
    let state = app.state::<AppState>();

    let (mount_count, mounted_buckets, has_mounts) = {
        let mounts = state.mounts.lock().unwrap();
        let count = mounts.len();
        let buckets: Vec<String> = mounts.iter().map(|m| m.bucket.clone()).collect();
        (count, buckets, count > 0)
    };

    let autostart_enabled = app.autolaunch().is_enabled().unwrap_or(false);

    let show_i = MenuItem::with_id(app, "show", "显示窗口", true, None::<&str>)?;
    let status_text = if mount_count > 0 {
        format!("状态: {} 个 Bucket 已挂载", mount_count)
    } else {
        "状态: 未连接".to_string()
    };
    let status_i = MenuItem::with_id(app, "status", &status_text, false, None::<&str>)?;
    let mount_i = MenuItem::with_id(app, "mount", "快速挂载", true, None::<&str>)?;
    let unmount_i = MenuItem::with_id(app, "unmount", "卸载全部", has_mounts, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;

    let autostart_label = if autostart_enabled {
        "开机自启  \u{2713}"
    } else {
        "开机自启"
    };
    let autostart_i = MenuItem::with_id(app, "autostart", autostart_label, true, None::<&str>)?;

    let mounted_menu = if has_mounts {
        let mut builder = SubmenuBuilder::new(app, "已挂载 Bucket");
        for bucket in &mounted_buckets {
            let item = MenuItemBuilder::new(format!("{}  \u{2716}", bucket))
                .id(format!("unmount:{}", bucket))
                .enabled(true)
                .build(app)?;
            builder = builder.item(&item);
        }
        Some(builder.build()?)
    } else {
        None
    };

    let mut builder = MenuBuilder::new(app)
        .item(&show_i)
        .item(&status_i)
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&mount_i)
        .item(&unmount_i)
        .item(&autostart_i)
        .item(&PredefinedMenuItem::separator(app)?)
        .item(&quit_i);

    if let Some(ref submenu) = mounted_menu {
        builder = builder.item(submenu);
    }

    builder.build().map_err(|e| e.into())
}
