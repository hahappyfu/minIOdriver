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
export type MountStatus = 'disconnected' | 'connecting' | 'connected' | 'error'
