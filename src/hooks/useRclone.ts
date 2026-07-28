import { invoke } from '@tauri-apps/api/core'
import type { Connection, Bucket, MountInfo, CacheConfig } from '../types'

// 连接管理
export async function testConnection(endpoint: string, accessKey: string, secretKey: string): Promise<boolean> {
  return await invoke('test_connection', { endpoint, accessKey, secretKey })
}

export async function saveConnection(connection: Connection): Promise<void> {
  await invoke('save_connection', { connection })
}

export async function loadConnections(): Promise<Connection[]> {
  return await invoke('load_connections')
}

export async function deleteConnection(id: string): Promise<void> {
  await invoke('delete_connection', { id })
}

export async function setCurrentConnection(id: string): Promise<void> {
  await invoke('set_current_connection', { id })
}

// Bucket 管理
export async function listBuckets(): Promise<Bucket[]> {
  return await invoke('list_buckets')
}

// 挂载管理
export async function mountBucket(bucket: string, drive: string): Promise<string> {
  return await invoke('mount_bucket', { bucket, drive })
}

export async function unmountBucket(bucket: string): Promise<void> {
  await invoke('unmount_bucket', { bucket })
}

export async function unmountAll(): Promise<void> {
  await invoke('unmount_all')
}

export async function getMountStatus(): Promise<MountInfo[]> {
  return await invoke('get_mount_status')
}

// 系统功能
export async function checkWinfsp(): Promise<boolean> {
  return await invoke('check_winfsp')
}

export async function setAutostart(enable: boolean): Promise<void> {
  await invoke('set_autostart', { enable })
}

export async function getAutostart(): Promise<boolean> {
  return await invoke('get_autostart')
}

export async function exportConfig(path: string): Promise<void> {
  await invoke('export_config', { path })
}

export async function importConfig(path: string): Promise<void> {
  await invoke('import_config', { path })
}

// 缓存配置
export async function getCacheConfig(): Promise<CacheConfig> {
  return await invoke('get_cache_config')
}

export async function saveCacheConfig(config: CacheConfig): Promise<void> {
  await invoke('save_cache_config', { config })
}

export async function clearCache(config?: CacheConfig): Promise<string> {
  return await invoke('clear_cache', { config })
}
