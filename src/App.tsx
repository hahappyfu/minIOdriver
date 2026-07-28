import React, { useState } from 'react'
import { Layout, Menu, theme } from 'antd'
import {
  CloudServerOutlined,
  DatabaseOutlined,
  SettingOutlined,
} from '@ant-design/icons'
import ConnectionConfig from './components/ConnectionConfig'
import BucketList from './components/BucketList'
import Settings from './components/Settings'

const { Content, Sider } = Layout

const App: React.FC = () => {
  const [selectedConnection, setSelectedConnection] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState('connections')

  const {
    token: { colorBgContainer, borderRadiusLG },
  } = theme.useToken()

  const menuItems = [
    {
      key: 'connections',
      icon: <CloudServerOutlined />,
      label: '连接配置',
    },
    {
      key: 'buckets',
      icon: <DatabaseOutlined />,
      label: 'Bucket 管理',
    },
    {
      key: 'settings',
      icon: <SettingOutlined />,
      label: '设置',
    },
  ]

  const renderContent = () => {
    switch (activeTab) {
      case 'connections':
        return <ConnectionConfig onConnectionSelect={setSelectedConnection} />
      case 'buckets':
        return <BucketList connectionId={selectedConnection} />
      case 'settings':
        return <Settings />
      default:
        return null
    }
  }

  return (
    <Layout style={{ minHeight: '100vh' }}>
      <Sider width={200} style={{ background: colorBgContainer }}>
        <div style={{ height: 32, margin: 16, textAlign: 'center' }}>
          <h2 style={{ margin: 0, color: '#1890ff' }}>MinIO Drive</h2>
        </div>
        <Menu
          mode="inline"
          selectedKeys={[activeTab]}
          items={menuItems}
          onClick={({ key }) => setActiveTab(key)}
          style={{ height: '100%', borderRight: 0 }}
        />
      </Sider>
      <Layout style={{ padding: '24px' }}>
        <Content
          style={{
            padding: 24,
            margin: 0,
            background: colorBgContainer,
            borderRadius: borderRadiusLG,
          }}
        >
          {renderContent()}
        </Content>
      </Layout>
    </Layout>
  )
}

export default App
