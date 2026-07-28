use anyhow::{Context, Result};
use keyring;
use serde_json;
use std::fs;
use std::path::PathBuf;

use crate::Connection;

/// 获取配置文件路径
fn get_config_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Failed to get config directory")?
        .join("minio-drive");

    fs::create_dir_all(&config_dir)?;

    Ok(config_dir.join("connections.json"))
}

/// 保存连接配置到文件
pub fn save_connections(connections: &[Connection]) -> Result<()> {
    let path = get_config_path()?;
    let json = serde_json::to_string_pretty(connections)?;
    fs::write(&path, json)?;
    Ok(())
}

/// 从文件加载连接配置
pub fn load_connections() -> Result<Vec<Connection>> {
    let path = get_config_path()?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let json = fs::read_to_string(&path)?;
    let connections: Vec<Connection> = serde_json::from_str(&json)?;
    Ok(connections)
}

/// 保存凭证到 Windows Credential Manager
pub fn save_credential(service: &str, username: &str, password: &str) -> Result<()> {
    let entry = keyring::Entry::new(service, username)?;
    entry.set_password(password)?;
    Ok(())
}

/// 从 Windows Credential Manager 读取凭证
pub fn load_credential(service: &str, username: &str) -> Result<String> {
    let entry = keyring::Entry::new(service, username)?;
    let password = entry.get_password()?;
    Ok(password)
}

/// 删除凭证
pub fn delete_credential(service: &str, username: &str) -> Result<()> {
    let entry = keyring::Entry::new(service, username)?;
    entry.delete_credential()?;
    Ok(())
}

/// 导出配置到文件
pub fn export_to_file(path: &str) -> Result<()> {
    let connections = load_connections()?;
    let json = serde_json::to_string_pretty(&connections)?;
    fs::write(path, json)?;
    Ok(())
}

/// 从文件导入配置
pub fn import_from_file(path: &str) -> Result<()> {
    let json = fs::read_to_string(path)?;
    let connections: Vec<Connection> = serde_json::from_str(&json)?;
    save_connections(&connections)?;
    Ok(())
}
