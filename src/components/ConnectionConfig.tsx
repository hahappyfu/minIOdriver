import React, { useState, useEffect } from 'react'
import {
  Card,
  Form,
  Input,
  Button,
  List,
  Modal,
  message,
  Space,
  Tag,
  Alert,
  Spin,
  Typography,
  Popconfirm,
  Tooltip,
  Badge,
} from 'antd'
import {
  PlusOutlined,
  DeleteOutlined,
  CheckCircleOutlined,
  CloseCircleOutlined,
  LoadingOutlined,
  ExclamationCircleOutlined,
  EyeInvisibleOutlined,
  EyeTwoTone,
} from '@ant-design/icons'
import type { Connection } from '../types'
import { loadConnections, saveConnection, deleteConnection, testConnection, setCurrentConnection } from '../hooks/useRclone'

interface Props {
  onConnectionSelect: (id: string) => void
}

const { Text } = Typography

const ConnectionConfig: React.FC<Props> = ({ onConnectionSelect }) => {
  const [connections, setConnections] = useState<Connection[]>([])
  const [isModalOpen, setIsModalOpen] = useState(false)
  const [testing, setTesting] = useState(false)
  const [loading, setLoading] = useState(false)
  const [testStatus, setTestStatus] = useState<'idle' | 'success' | 'error' | 'testing'>('idle')
  const [form] = Form.useForm()

  useEffect(() => {
    loadConnectionsList()
  }, [])

  const loadConnectionsList = async () => {
    setLoading(true)
    try {
      const list = await loadConnections()
      setConnections(list)
    } catch (error) {
      console.error('加载连接失败:', error)
      message.error('加载连接列表失败')
    } finally {
      setLoading(false)
    }
  }

  const handleAdd = async (values: any) => {
    const connection: Connection = {
      id: Date.now().toString(),
      name: values.name,
      endpoint: values.endpoint,
      access_key: values.accessKey,
      secret_key: values.secretKey,
    }

    try {
      await saveConnection(connection)
      await loadConnectionsList()
      setIsModalOpen(false)
      form.resetFields()
      setTestStatus('idle')
      message.success('连接已保存')
    } catch (error) {
      message.error('保存失败')
    }
  }

  const handleDelete = async (id: string) => {
    try {
      await deleteConnection(id)
      await loadConnectionsList()
      message.success('连接已删除')
    } catch (error) {
      message.error('删除失败')
    }
  }

  const handleTest = async () => {
    try {
      const values = await form.validateFields()
      setTesting(true)
      setTestStatus('testing')
      const success = await testConnection(values.endpoint, values.accessKey, values.secretKey)
      if (success) {
        setTestStatus('success')
        message.success('连接测试成功')
      } else {
        setTestStatus('error')
        message.error('连接测试失败')
      }
    } catch (error) {
      setTestStatus('error')
      message.error('请填写完整信息')
    } finally {
      setTesting(false)
    }
  }

  const handleSelect = async (id: string) => {
    try {
      await setCurrentConnection(id)
      onConnectionSelect(id)
      message.success('已选择连接')
    } catch (error) {
      message.error('选择失败')
    }
  }

  const getStatusIcon = (status: 'idle' | 'success' | 'error' | 'testing') => {
    switch (status) {
      case 'success':
        return <CheckCircleOutlined style={{ color: '#52c41a', fontSize: 16 }} />
      case 'error':
        return <CloseCircleOutlined style={{ color: '#ff4d4f', fontSize: 16 }} />
      case 'testing':
        return <LoadingOutlined style={{ color: '#1890ff', fontSize: 16 }} />
      default:
        return null
    }
  }

  // URL 验证规则
  const validateUrl = (_: any, value: string) => {
    if (!value) {
      return Promise.reject(new Error('请输入服务器地址'))
    }
    try {
      const url = new URL(value)
      if (!['http:', 'https:'].includes(url.protocol)) {
        return Promise.reject(new Error('仅支持 HTTP 和 HTTPS 协议'))
      }
      if (!url.hostname) {
        return Promise.reject(new Error('请输入有效的服务器地址'))
      }
      return Promise.resolve()
    } catch {
      return Promise.reject(new Error('请输入有效的 URL 格式（如：http://minio-server:9000）'))
    }
  }

  // 重置测试状态
  const handleModalClose = () => {
    setIsModalOpen(false)
    form.resetFields()
    setTestStatus('idle')
  }

  return (
    <Card
      title="连接配置"
      extra={
        <Space>
          <Tooltip title="刷新连接列表">
            <Button onClick={loadConnectionsList} loading={loading}>
              刷新
            </Button>
          </Tooltip>
          <Button type="primary" icon={<PlusOutlined />} onClick={() => setIsModalOpen(true)}>
            添加连接
          </Button>
        </Space>
      }
    >
      <Spin spinning={loading}>
        {connections.length === 0 && !loading && (
          <Alert
            message="暂无连接配置"
            description={'点击"添加连接"按钮来创建你的第一个 MinIO 连接'}
            type="info"
            showIcon
            style={{ marginBottom: 16 }}
          />
        )}

        <List
          dataSource={connections}
          renderItem={(item) => (
            <List.Item
              actions={[
                <Button type="primary" size="small" onClick={() => handleSelect(item.id)}>
                  选择连接
                </Button>,
                <Popconfirm
                  title="确定要删除这个连接吗？"
                  description="删除后无法恢复"
                  onConfirm={() => handleDelete(item.id)}
                  okText="确定"
                  cancelText="取消"
                  okType="danger"
                >
                  <Button type="link" danger icon={<DeleteOutlined />}>
                    删除
                  </Button>
                </Popconfirm>,
              ]}
            >
              <List.Item.Meta
                title={
                  <Space>
                    <Text strong>{item.name}</Text>
                    <Badge status="success" text="" />
                  </Space>
                }
                description={
                  <Space direction="vertical" size={4}>
                    <Space size={4}>
                      <Tag color="blue">Endpoint</Tag>
                      <Text code copyable>{item.endpoint}</Text>
                    </Space>
                    <Space size={4}>
                      <Tag color="orange">Access Key</Tag>
                      <Text code>{item.access_key.substring(0, 6)}****</Text>
                    </Space>
                  </Space>
                }
              />
            </List.Item>
          )}
        />
      </Spin>

      <Modal
        title={
          <Space>
            <span>添加 MinIO 连接</span>
            {testStatus !== 'idle' && getStatusIcon(testStatus)}
          </Space>
        }
        open={isModalOpen}
        onCancel={handleModalClose}
        footer={null}
        destroyOnClose
        width={520}
      >
        <Form
          form={form}
          layout="vertical"
          onFinish={handleAdd}
          validateTrigger={['onChange', 'onBlur']}
          size="large"
        >
          <Form.Item
            name="name"
            label="连接名称"
            rules={[
              { required: true, message: '请输入连接名称' },
              { min: 2, message: '连接名称至少 2 个字符' },
              { max: 32, message: '连接名称最多 32 个字符' },
            ]}
          >
            <Input placeholder="例如：生产环境 MinIO" maxLength={32} showCount />
          </Form.Item>

          <Form.Item
            name="endpoint"
            label="服务器地址"
            rules={[
              { required: true, message: '请输入服务器地址' },
              { validator: validateUrl },
            ]}
            extra="支持 HTTP 和 HTTPS 协议，格式：http://服务器地址:端口"
          >
            <Input placeholder="http://minio-server:9000" />
          </Form.Item>

          <Form.Item
            name="accessKey"
            label="Access Key"
            rules={[
              { required: true, message: '请输入 Access Key' },
              { min: 4, message: 'Access Key 长度至少 4 个字符' },
            ]}
          >
            <Input placeholder="请输入 Access Key" />
          </Form.Item>

          <Form.Item
            name="secretKey"
            label="Secret Key"
            rules={[
              { required: true, message: '请输入 Secret Key' },
              { min: 4, message: 'Secret Key 长度至少 4 个字符' },
            ]}
          >
            <Input.Password
              placeholder="请输入 Secret Key"
              iconRender={(visible) =>
                visible ? <EyeTwoTone /> : <EyeInvisibleOutlined />
              }
            />
          </Form.Item>

          {testStatus === 'success' && (
            <Alert
              message="连接测试成功"
              type="success"
              showIcon
              style={{ marginBottom: 16 }}
            />
          )}

          {testStatus === 'error' && (
            <Alert
              message="连接测试失败"
              description="请检查服务器地址、Access Key 和 Secret Key 是否正确，或网络是否通畅"
              type="error"
              showIcon
              style={{ marginBottom: 16 }}
            />
          )}

          <Form.Item style={{ marginBottom: 0 }}>
            <Space>
              <Button type="primary" htmlType="submit">
                保存
              </Button>
              <Button
                onClick={handleTest}
                loading={testing}
                icon={testStatus === 'success' ? <CheckCircleOutlined /> : testStatus === 'error' ? <ExclamationCircleOutlined /> : undefined}
              >
                测试连接
              </Button>
              <Button onClick={handleModalClose}>
                取消
              </Button>
            </Space>
          </Form.Item>
        </Form>
      </Modal>
    </Card>
  )
}

export default ConnectionConfig
