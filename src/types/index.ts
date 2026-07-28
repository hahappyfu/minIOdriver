// 连接配置
export interface Connection {
  id: string
  name: string
  endpoint: string
  access_key: string
  secret_key: string
}

// Bucket 信息
export interface Bucket {
  name: string
  drive: string | null
  mounted: boolean
}

// 挂载信息
export interface MountInfo {
  bucket: string
  drive: string
  status: string
}

// 挂载状态类型
export type MountStatus = 'disconnected' | 'connecting' | 'connected' | 'reconnecting' | 'error'

// 缓存配置
export interface CacheConfig {
  cache_dir: string      // 缓存目录路径
  max_size_gb: number    // 缓存大小上限（GB）
  max_age_hours: number  // 缓存过期时间（小时）
}
