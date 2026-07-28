import React, { useState, useEffect } from 'react'
import { Card, Switch, Button, message, Space, Alert, Typography } from 'antd'
import { DownloadOutlined, UploadOutlined, CheckCircleOutlined, WarningOutlined } from '@ant-design/icons'
import { checkWinfsp, setAutostart, getAutostart, exportConfig, importConfig } from '../hooks/useRclone'

const { Text } = Typography

const Settings: React.FC = () => {
  const [winfspInstalled, setWinfspInstalled] = useState<boolean | null>(null)
  const [autostartEnabled, setAutostartEnabled] = useState(false)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    loadSettings()
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
    </Space>
  )
}

export default Settings
