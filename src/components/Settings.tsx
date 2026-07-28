import React, { useState, useEffect } from 'react'
import { Card, Switch, Button, message, Space, Alert, Typography, Input, InputNumber, Modal, Form } from 'antd'
import { DownloadOutlined, UploadOutlined, CheckCircleOutlined, WarningOutlined, ClearOutlined, SaveOutlined } from '@ant-design/icons'
import { checkWinfsp, setAutostart, getAutostart, exportConfig, importConfig, getCacheConfig, saveCacheConfig, clearCache } from '../hooks/useRclone'
import type { CacheConfig } from '../types'

const { Text } = Typography

const Settings: React.FC = () => {
  const [winfspInstalled, setWinfspInstalled] = useState<boolean | null>(null)
  const [autostartEnabled, setAutostartEnabled] = useState(false)
  const [loading, setLoading] = useState(true)
  const [cacheConfig, setCacheConfig] = useState<CacheConfig>({
    cache_dir: '~/.cache/rclone',
    max_size_gb: 10,
    max_age_hours: 1,
  })
  const [cacheLoading, setCacheLoading] = useState(false)

  useEffect(() => {
    loadSettings()
    loadCacheSettings()
  }, [])

  const loadSettings = async () => {
    setLoading(true)
    try {
      const [winfsp, autostart] = await Promise.all([
        checkWinfsp(),
        getAutostart(),
      ])
      setWinfspInstalled(winfsp)
      setAutostartEnabled(autostart)
    } catch (error) {
      console.error('加载设置失败:', error)
    } finally {
      setLoading(false)
    }
  }

  const loadCacheSettings = async () => {
    try {
      const config = await getCacheConfig()
      setCacheConfig(config)
    } catch (error) {
      console.error('加载缓存配置失败:', error)
    }
  }

  const handleAutostartChange = async (checked: boolean) => {
    try {
      await setAutostart(checked)
      setAutostartEnabled(checked)
      message.success(checked ? '已开启开机自启' : '已关闭开机自启')
    } catch (error) {
      message.error('设置失败')
    }
  }

  const handleExport = async () => {
    try {
      // TODO: 使用文件对话框选择路径
      await exportConfig('minio-drive-config.json')
      message.success('配置已导出')
    } catch (error) {
      message.error('导出失败')
    }
  }

  const handleImport = async () => {
    try {
      // TODO: 使用文件对话框选择文件
      await importConfig('minio-drive-config.json')
      message.success('配置已导入')
    } catch (error) {
      message.error('导入失败')
    }
  }

  const handleSaveCacheConfig = async () => {
    try {
      await saveCacheConfig(cacheConfig)
      message.success('缓存配置已保存')
    } catch (error) {
      message.error('保存失败')
    }
  }

  const handleClearCache = () => {
    Modal.confirm({
      title: '确认清理缓存',
      content: '将删除所有 rclone VFS 缓存文件，此操作不可恢复。是否继续？',
      okText: '确认清理',
      okType: 'danger',
      cancelText: '取消',
      onOk: async () => {
        setCacheLoading(true)
        try {
          const result = await clearCache(cacheConfig)
          message.success(result)
        } catch (error) {
          message.error('清理失败')
        } finally {
          setCacheLoading(false)
        }
      },
    })
  }

  return (
    <Space direction="vertical" style={{ width: '100%' }} size="large">
      {/* WinFsp 状态 */}
      <Card title="系统依赖">
        {winfspInstalled === null ? (
          <Text>检测中...</Text>
        ) : winfspInstalled ? (
          <Alert
            message="WinFsp 已安装"
            description="rclone 挂载功能正常可用"
            type="success"
            showIcon
            icon={<CheckCircleOutlined />}
          />
        ) : (
          <Alert
            message="WinFsp 未安装"
            description={
              <span>
                rclone 挂载依赖 WinFsp，请先安装。
                <a href="https://winfsp.dev/rel/" target="_blank" rel="noopener noreferrer">
                  点击下载 WinFsp
                </a>
              </span>
            }
            type="warning"
            showIcon
            icon={<WarningOutlined />}
          />
        )}
      </Card>

      {/* 开机自启 */}
      <Card title="启动设置">
        <Space>
          <Text>开机自动启动：</Text>
          <Switch
            checked={autostartEnabled}
            onChange={handleAutostartChange}
            loading={loading}
          />
        </Space>
        <div style={{ marginTop: 8, color: '#999' }}>
          开启后，系统启动时将自动运行 MinIO Drive 并挂载已配置的 Bucket
        </div>
      </Card>

      {/* 配置导入导出 */}
      <Card title="配置管理">
        <Space>
          <Button icon={<DownloadOutlined />} onClick={handleExport}>
            导出配置
          </Button>
          <Button icon={<UploadOutlined />} onClick={handleImport}>
            导入配置
          </Button>
        </Space>
        <div style={{ marginTop: 8, color: '#999' }}>
          导出/导入连接配置，方便在多台电脑间迁移
        </div>
      </Card>

      {/* 缓存配置 */}
      <Card title="缓存设置">
        <Form layout="vertical">
          <Form.Item label="缓存目录">
            <Input
              value={cacheConfig.cache_dir}
              onChange={(e) => setCacheConfig({ ...cacheConfig, cache_dir: e.target.value })}
              placeholder="~/.cache/rclone"
            />
            <div style={{ color: '#999', fontSize: 12 }}>
              rclone VFS 缓存存储路径，支持 ~ 开头的相对路径
            </div>
          </Form.Item>
          <Form.Item label="缓存大小上限 (GB)">
            <InputNumber
              min={1}
              max={1000}
              value={cacheConfig.max_size_gb}
              onChange={(value) => setCacheConfig({ ...cacheConfig, max_size_gb: value || 10 })}
              addonAfter="GB"
              style={{ width: '100%' }}
            />
          </Form.Item>
          <Form.Item label="缓存过期时间 (小时)">
            <InputNumber
              min={1}
              max={8760}
              value={cacheConfig.max_age_hours}
              onChange={(value) => setCacheConfig({ ...cacheConfig, max_age_hours: value || 1 })}
              addonAfter="小时"
              style={{ width: '100%' }}
            />
          </Form.Item>
          <Space>
            <Button
              type="primary"
              icon={<SaveOutlined />}
              onClick={handleSaveCacheConfig}
            >
              保存配置
            </Button>
            <Button
              danger
              icon={<ClearOutlined />}
              loading={cacheLoading}
              onClick={handleClearCache}
            >
              清理缓存
            </Button>
          </Space>
        </Form>
        <div style={{ marginTop: 8, color: '#999' }}>
          调整缓存设置后，新挂载的 Bucket 将使用新配置
        </div>
      </Card>
    </Space>
  )
}

export default Settings
