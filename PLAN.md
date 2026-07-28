# MinIO Drive - 实施计划

## 项目概述

**MinIO Drive** 是一个 Windows 托盘应用程序，用于管理 MinIO 存储桶的挂载。它将 MinIO bucket 映射为 Windows 盘符，让用户在资源管理器中像操作本地磁盘一样操作 MinIO 文件（支持 CV、拖拽等原生操作）。

## 技术架构

```
┌─────────────────────────────────────────────────────┐
│                   Windows 资源管理器                   │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐              │
│  │ 本地磁盘 │  │ MinIO:  │  │ MinIO:  │  ← 挂载后的盘符│
│  │   C:    │  │   E:    │  │   F:    │   完全原生体验  │
│  └─────────┘  └─────────┘  └─────────┘   支持 CV/拖拽 │
└─────────────────────────────────────────────────────┘
                             ▲
                             │ 挂载管理
┌─────────────────────────────────────────────────────┐
│                   Tauri 托盘程序（后台）                │
│  ┌─────────────────────────────────────────────────┐│
│  │ • MinIO 连接配置（地址、AK/SK）                   ││
│  │ • Bucket 列表与盘符映射                           ││
│  │ • 一键挂载/卸载                                   ││
│  │ • 开机自动挂载                                    ││
│  │ • 状态监控（连接状态、传输速度）                   ││
│  └─────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────┘
                             │
                             ▼
                       ┌──────────┐
                       │  rclone  │  ← 底层挂载引擎（Sidecar）
                       └──────────┘
                             │
                             ▼
                       ┌──────────┐
                       │  MinIO   │
                       └──────────┘
```

## 技术栈

| 组件 | 技术选择 | 说明 |
|------|---------|------|
| **桌面框架** | Tauri 2.x | 轻量、原生、性能好 |
| **前端** | React + TypeScript | 组件化开发，生态成熟 |
| **UI 组件库** | Ant Design / Shadcn/ui | 简洁实用风格 |
| **后端** | Rust | Tauri 原生后端，管理进程和凭证 |
| **挂载引擎** | rclone (Sidecar) | 内置二进制，无需用户安装 |
| **凭证存储** | keyring crate | Windows Credential Manager |
| **自启动** | tauri-plugin-autostart | 开机自动启动 |

## 功能模块

### 1. 连接配置管理
- [ ] MinIO 服务器地址输入
- [ ] Access Key / Secret Key 输入（密码框）
- [ ] 连接测试功能
- [ ] 多连接配置支持（保存/切换）
- [ ] 配置导入/导出（JSON 格式）

### 2. Bucket 挂载管理
- [ ] 自动获取 Bucket 列表
- [ ] 为每个 Bucket 分配盘符（A-Z 下拉选择）
- [ ] 一键挂载/卸载单个 Bucket
- [ ] 批量挂载/卸载所有 Bucket
- [ ] 挂载参数配置（缓存大小、写回延迟等）

### 3. 状态监控
- [ ] 连接状态显示（已连接/连接中/断开/错误）
- [ ] 挂载状态显示（已挂载/未挂载）
- [ ] 传输速度显示（可选，通过 rclone RC API）
- [ ] 错误日志查看

### 4. 系统托盘
- [ ] 托盘图标（显示连接状态）
- [ ] 右键菜单：
  - 显示主窗口
  - 挂载状态（只读）
  - 快速挂载/卸载
  - 开机自启开关
  - 退出

### 5. 开机自启
- [ ] 开机自启开关
- [ ] 自启时自动挂载已配置的 Bucket
- [ ] 静默启动（最小化到托盘）

### 6. 环境检测
- [ ] WinFsp 安装检测
- [ ] WinFsp 下载引导
- [ ] rclone 版本检测

## 实施步骤

### 阶段一：项目初始化（Day 1）
1. 创建 Tauri 2.x 项目
2. 配置前端框架（React + TypeScript）
3. 集成 UI 组件库
4. 配置开发环境（热重载等）

### 阶段二：核心功能 - 连接配置（Day 2-3）
1. 实现连接配置 UI
2. 实现凭证存储（keyring）
3. 实现配置导入/导出
4. 实现连接测试

### 阶段三：核心功能 - 挂载管理（Day 4-5）
1. 集成 rclone Sidecar
2. 实现 Bucket 列表获取
3. 实现挂载/卸载功能
4. 实现进程管理和状态监控

### 阶段四：系统集成（Day 6）
1. 实现系统托盘
2. 实现开机自启
3. 实现 WinFsp 检测
4. 实现静默启动

### 阶段五：优化和完善（Day 7）
1. 错误处理完善
2. 用户体验优化
3. 配置参数高级选项
4. 测试和调试

## 关键代码结构

```
minio-drive/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs              # Tauri 入口
│   │   ├── lib.rs               # 核心逻辑
│   │   ├── rclone.rs            # rclone 进程管理
│   │   ├── credentials.rs       # 凭证管理
│   │   ├── config.rs            # 配置管理
│   │   └── tray.rs              # 系统托盘
│   ├── binaries/
│   │   └── rclone-x86_64-pc-windows-msvc.exe  # rclone 二进制
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── App.tsx
│   ├── components/
│   │   ├── ConnectionConfig.tsx  # 连接配置
│   │   ├── BucketList.tsx       # Bucket 列表
│   │   ├── MountStatus.tsx      # 挂载状态
│   │   └── Settings.tsx         # 设置页面
│   ├── hooks/
│   │   └── useRclone.ts         # rclone 操作 hook
│   └── types/
│       └── index.ts             # 类型定义
├── package.json
└── README.md
```

## Tauri 命令（Rust ↔ JS 桥接）

```rust
// 连接管理
#[tauri::command] async fn test_connection(endpoint, ak, sk) -> Result<bool, String>
#[tauri::command] async fn save_connection(config) -> Result<(), String>
#[tauri::command] async fn load_connections() -> Result<Vec<Connection>, String>
#[tauri::command] async fn delete_connection(id) -> Result<(), String>

// Bucket 管理
#[tauri::command] async fn list_buckets(connection_id) -> Result<Vec<Bucket>, String>

// 挂载管理
#[tauri::command] async fn mount_bucket(bucket, drive) -> Result<String, String>
#[tauri::command] async fn unmount_bucket(bucket) -> Result<(), String>
#[tauri::command] async fn unmount_all() -> Result<(), String>
#[tauri::command] async fn get_mount_status() -> Result<Vec<MountInfo>, String>

// 系统功能
#[tauri::command] async fn check_winfsp() -> Result<bool, String>
#[tauri::command] async fn set_autostart(enable) -> Result<(), String>
#[tauri::command] async fn get_autostart() -> Result<bool, String>

// 配置导入导出
#[tauri::command] async fn export_config(path) -> Result<(), String>
#[tauri::command] async fn import_config(path) -> Result<(), String>
```

## rclone 命令示例

```bash
# 列出 bucket
rclone lsd minio: --minio-endpoint http://localhost:9000

# 挂载 bucket
rclone mount minio:mybucket X: \
  --vfs-cache-mode full \
  --volname "MinIO-mybucket" \
  --vfs-cache-max-size 10G \
  --vfs-cache-max-age 1h \
  --log-level INFO

# 使用 RC API 监控
rclone mount minio:mybucket X: \
  --vfs-cache-mode full \
  --rc --rc-addr localhost:5572

# 获取状态
curl http://localhost:5572/core/stats
```

## 风险和注意事项

1. **WinFsp 硬依赖**
   - 首次使用需要安装 WinFsp
   - 解决方案：应用内检测并引导下载

2. **rclone 二进制体积**
   - 约 50-60MB，增大安装包
   - 解决方案：首次运行时下载，或作为可选组件

3. **Windows 无 daemon 模式**
   - rclone mount 必须前台运行
   - 解决方案：Tauri 子进程管理，保持句柄不释放

4. **多实例冲突**
   - 同一 Bucket 不能同时挂载多次
   - 解决方案：应用层去重检查

5. **凭证安全**
   - 使用 Windows Credential Manager 加密存储
   - 内存中临时明文用完即清

## 成功标准

- [ ] 用户可以配置 MinIO 连接（地址 + AK/SK）
- [ ] 用户可以看到 Bucket 列表
- [ ] 用户可以一键挂载 Bucket 为盘符
- [ ] 在资源管理器中可以正常访问挂载的盘符
- [ ] 支持文件的复制、粘贴、拖拽等原生操作
- [ ] 支持开机自动挂载
- [ ] 托盘图标显示连接状态
- [ ] WinFsp 未安装时有引导提示

## 下一步

确认计划后，我将开始实施阶段一：项目初始化。
