import React, { useState, useEffect } from 'react'
import { Card, Table, Select, Button, message, Space, Tag } from 'antd'
import { PlayCircleOutlined, PauseCircleOutlined, ReloadOutlined } from '@ant-design/icons'
import type { Bucket, MountInfo } from '../types'
import { listBuckets, mountBucket, unmountBucket, getMountStatus } from '../hooks/useRclone'

interface Props {
  connectionId: string | null
}

// 可用盘符
const AVAILABLE_DRIVES = Array.from({ length: 26 }, (_, i) => String.fromCharCode(65 + i))
  .filter(d => !['A', 'B', 'C', 'D'].includes(d)) // 排除系统盘符

const BucketList: React.FC<Props> = ({ connectionId }) => {
  const [buckets, setBuckets] = useState<Bucket[]>([])
  const [mounts, setMounts] = useState<MountInfo[]>([])
  const [loading, setLoading] = useState(false)
  const [selectedDrives, setSelectedDrives] = useState<Record<string, string>>({})

  useEffect(() => {
    if (connectionId) {
      loadData()
    }
  }, [connectionId])

  const loadData = async () => {
    setLoading(true)
    try {
      const [bucketList, mountList] = await Promise.all([
        listBuckets(),
        getMountStatus(),
      ])
      setBuckets(bucketList)
      setMounts(mountList)

      // 初始化已挂载的盘符
      const driveMap: Record<string, string> = {}
      mountList.forEach(m => {
        driveMap[m.bucket] = m.drive
      })
      setSelectedDrives(driveMap)
    } catch (error) {
      console.error('加载数据失败:', error)
    } finally {
      setLoading(false)
    }
  }

  const handleMount = async (bucketName: string) => {
    const drive = selectedDrives[bucketName]
    if (!drive) {
      message.warning('请先选择盘符')
      return
    }

    try {
      await mountBucket(bucketName, drive)
      message.success(`正在挂载 ${bucketName} 到 ${drive}:`)
      // 刷新状态
      setTimeout(loadData, 1000)
    } catch (error: any) {
      message.error(error.toString())
    }
  }

  const handleUnmount = async (bucketName: string) => {
    try {
      await unmountBucket(bucketName)
      message.success(`已卸载 ${bucketName}`)
      await loadData()
    } catch (error: any) {
      message.error(error.toString())
    }
  }

  const columns = [
    {
      title: 'Bucket 名称',
      dataIndex: 'name',
      key: 'name',
    },
    {
      title: '盘符',
      key: 'drive',
      render: (_: any, record: Bucket) => {
        const isMounted = record.mounted
        const mount = mounts.find(m => m.bucket === record.name)

        if (isMounted && mount) {
          return <Tag color="success">{mount.drive}:</Tag>
        }

        return (
          <Select
            style={{ width: 80 }}
            placeholder="选择"
            value={selectedDrives[record.name]}
            onChange={(value) => setSelectedDrives(prev => ({ ...prev, [record.name]: value }))}
            options={AVAILABLE_DRIVES.map(d => ({ label: `${d}:`, value: d }))}
          />
        )
      },
    },
    {
      title: '状态',
      key: 'status',
      render: (_: any, record: Bucket) => {
        const mount = mounts.find(m => m.bucket === record.name)
        if (!mount) return <Tag>未挂载</Tag>

        const statusMap: Record<string, { color: string; text: string }> = {
          mounting: { color: 'processing', text: '挂载中' },
          mounted: { color: 'success', text: '已挂载' },
          error: { color: 'error', text: '错误' },
        }

        const status = statusMap[mount.status] || { color: 'default', text: mount.status }
        return <Tag color={status.color}>{status.text}</Tag>
      },
    },
    {
      title: '操作',
      key: 'action',
      render: (_: any, record: Bucket) => {
        const isMounted = record.mounted

        return (
          <Space>
            {isMounted ? (
              <Button
                type="link"
                danger
                icon={<PauseCircleOutlined />}
                onClick={() => handleUnmount(record.name)}
              >
                卸载
              </Button>
            ) : (
              <Button
                type="link"
                icon={<PlayCircleOutlined />}
                onClick={() => handleMount(record.name)}
              >
                挂载
              </Button>
            )}
          </Space>
        )
      },
    },
  ]

  if (!connectionId) {
    return (
      <Card title="Bucket 列表">
        <div style={{ textAlign: 'center', color: '#999' }}>请先选择一个连接</div>
      </Card>
    )
  }

  return (
    <Card
      title="Bucket 列表"
      extra={
        <Button icon={<ReloadOutlined />} onClick={loadData}>
          刷新
        </Button>
      }
    >
      <Table
        columns={columns}
        dataSource={buckets}
        rowKey="name"
        loading={loading}
        pagination={false}
      />
    </Card>
  )
}

export default BucketList
