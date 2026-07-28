# MinIO Drive - 项目交接文档

## 项目概述

MinIO Drive 是一个 Windows 桌面工具，通过 rclone 将 MinIO bucket 挂载为本地盘符，让用户在 Windows 资源管理器中像操作本地磁盘一样操作 MinIO 文件（复制、粘贴、拖拽）。

## 技术栈

| 组件 | 技术 |
|------|------|
| 桌面框架 | Tauri 2.x |
| 前端 | React + TypeScript + Ant Design |
| 后端 | Rust |
| 挂载引擎 | rclone (Sidecar) |
| 凭证存储 | keyring crate (Windows Credential Manager) |
| 自启动 | tauri-plugin-autostart |

## 项目地址

- **GitHub**: https://github.com/hahappyfu/minIOdriver.git
- **Gitee (旧)**: https://gitee.com/fuhahah/minIO_cli.git

## 已完成功能

### 1. 连接配置管理 ✅
- MinIO 服务器地址输入
- Access Key / Secret Key 输入
- 连接测试功能
- 多连接配置支持（保存/切换）
- 配置导入/导出（JSON 格式）
- 表单验证、加载状态、错误提示

### 2. Bucket 挂载管理 ✅
- 自动获取 Bucket 列表
- 为每个 Bucket 分配盘符（D-Z 下拉选择）
- 一键挂载/卸载单个 Bucket
- 多 bucket 并发挂载
- 挂载状态显示

### 3. 系统托盘 ✅
- 托盘图标（显示连接状态）
- 右键菜单：显示窗口、挂载状态、快速挂载/卸载、开机自启、退出
- 退出时自动清理所有 rclone 进程
- 窗口关闭按钮隐藏到托盘而非退出

### 4. 开机自启 ✅
- 开机自启开关
- 静默启动（最小化到托盘）

### 5. 缓存配置 ✅
- 自定义缓存目录
- 缓存大小上限设置（默认 10GB）
- 缓存过期时间设置（默认 1 小时）
- 一键清理缓存（带确认弹窗）
- 配置存储在 ~/.config/minio-drive/cache-config.json

### 6. 错误日志记录 ✅
- rclone 输出记录到日志文件
- 日志文件按日期命名：rclone-YYYYMMDD.log
- 日志目录：~/.config/minio-drive/logs/
- 同时保留控制台输出

### 7. 连接状态监控和自动重连 ✅
- 5 种挂载状态：connecting / connected / disconnected / reconnecting / error
- rclone 进程意外退出时自动重连（最多 3 次，间隔 3 秒）
- 前端实时状态显示（颜色 + 图标）
- 前端每 5 秒轮询状态
- 监听 Tauri 事件即时刷新

### 8. CI/CD ✅
- GitHub Actions 自动构建 Windows 安装包
- Release 已发布 v0.1.0

## 项目结构

```
minIOdriver/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs           # Tauri 入口
│   │   ├── lib.rs            # 核心逻辑 + Tauri 命令
│   │   ├── rclone.rs         # rclone 进程管理 ⭐ 核心
│   │   ├── credentials.rs    # 凭证管理
│   │   └── tray.rs           # 系统托盘
│   ├── binaries/
│   │   ├── rclone-aarch64-apple-darwin    # macOS ARM64
│   │   └── rclone-x86_64-pc-windows-msvc.exe  # Windows x64
│   ├── icons/
│   ├── capabilities/
│   │   └── default.json      # Tauri 权限配置
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── App.tsx               # 主应用（侧边栏布局）
│   ├── main.tsx              # 入口
│   ├── styles.css
│   ├── components/
│   │   ├── ConnectionConfig.tsx  # 连接配置组件
│   │   ├── BucketList.tsx       # Bucket 列表组件
│   │   └── Settings.tsx         # 设置页面
│   ├── hooks/
│   │   └── useRclone.ts      # Tauri 命令封装
│   └── types/
│       └── index.ts          # TypeScript 类型定义
├── .github/workflows/
│   └── build.yml             # CI/CD 配置
├── package.json
├── vite.config.ts
├── tsconfig.json
└── README.md
```

## 关键架构决策

### 为什么用 rclone 而不是直接调用 S3 API？

因为 rclone + WinFsp 可以把 MinIO bucket 挂载为 Windows 盘符，用户在资源管理器里直接操作，体验最好。

### rclone 进程管理

```
Tauri App (Rust 后端)
    ├── 生成 rclone.conf (临时配置文件)
    ├── 启动 rclone mount 子进程
    │   └── rclone mount minio:bucket X: --config /path/to/rclone.conf
    ├── 通过事件监听状态变化
    └── 退出时 kill 所有子进程
```

### 多 bucket 并发挂载

使用 `HashMap<String, RcloneProcess>` 管理多个 rclone 进程，每个 bucket 对应一个进程。

### 凭证存储

使用 `keyring` crate 将 AK/SK 存储在 Windows Credential Manager 中，安全加密。

## 已知问题

1. **rclone 路径问题**：`test_connection` 和 `list_buckets` 函数使用 `env!("CARGO_MANIFEST_DIR")` 拼接 rclone 路径，仅在开发模式有效。生产环境需要用 Tauri sidecar 方式调用。
2. **macOS 挂载需要 macFUSE**：当前未安装，挂载功能在 Mac 上不可用。
3. **Windows rclone 二进制较大**：75MB，导致仓库体积较大。

## 构建命令

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建生产版本
npm run tauri build

# 构建产物位置
# src-tauri/target/release/bundle/
```

## GitHub Actions

```yaml
# .github/workflows/build.yml
# 触发：push to main
# 产物：Windows MSI + EXE 安装包
# 发布：自动创建 GitHub Release (Draft)
```

## 在 Windows 上测试

1. 下载 Release: https://github.com/hahappyfu/minIOdriver/releases
2. 安装 [WinFsp](https://winfsp.dev/rel/)（rclone 挂载依赖）
3. 运行 MinIO Drive
4. 添加 MinIO 连接（地址: `http://127.0.0.1:9000`，AK: `minioadmin`，SK: `minioadmin`）
5. 选择 Bucket → 分配盘符 → 点击挂载
6. 在 Windows 资源管理器中访问挂载的盘符

## 后续可优化

### 已完成 ✅

1. ~~凭证安全存储~~ (已完成：keyring)
2. ~~系统托盘~~ (已完成)
3. ~~开机自启~~ (已完成)
4. ~~rclone 改用 sidecar 方式~~ (已完成：修复生产环境路径问题)
5. ~~缓存配置~~ (已完成：自定义目录、大小上限、过期时间、一键清理)
6. ~~错误日志记录~~ (已完成：按日期记录到 ~/.config/minio-drive/logs/)
7. ~~连接状态监控和自动重连~~ (已完成：5种状态显示、自动重连3次)

### 待做

8. 添加文件预览功能（在应用内查看文件内容）
9. 国际化（中英文切换）
10. 提取 sidecar 初始化为辅助函数，消除代码重复
11. 区分 sidecar 执行失败与连接测试失败的错误信息
12. 补充 macOS Intel 平台的 rclone 二进制文件

## Gemini 讨论记录（架构参考）

与 Gemini 进行了多轮深度讨论，以下是关键洞察：

### 竞品弱点
| 竞品 | 弱点 | 我们的机会 |
|------|------|-----------|
| OneDrive | 生态封闭，不支持私有化 MinIO | 专为 MinIO 优化 |
| Rclone UI | 只是 GUI 命令行生成器 | 一体化引擎 |
| NetDrive | POSIX 语义生硬翻译 | S3 原生赋能 |
| CarotDAV | 非后台无感挂载 | 系统级集成 |

### 进阶路线（未来可选）
- **V2**: rclone RC 守护进程统一纳管（单进程多挂载）
- **V3**: Windows CFAPI（OneDrive 级别体验）
- **杀手级功能**: 智能按需缓存 + Pin/Evict

### 核心架构洞察
- SQLite 邻接列表模型做目录树（百万级文件毫秒级 readdir）
- 分块缓存（5MB/块）+ 预读机制
- MinIO ListenNotification SSE 实时事件订阅
- 本地 Staging Area + 写时复制处理写操作

## 更新日志

### 2026-07-28 - 优化迭代

使用子智能体驱动开发（subagent-driven-development）完成 4 个优化任务：

| 任务 | 模型 | 审查结果 |
|------|------|---------|
| 修复 rclone 路径问题 | sonnet | ✅ 通过 |
| 添加缓存配置功能 | sonnet | ✅ 通过（修复了路径遍历安全问题） |
| 添加错误日志记录 | haiku | ✅ 通过 |
| 连接状态监控和自动重连 | sonnet | ✅ 通过（修复了状态时机和类型定义） |

**关键改进：**
- rclone 改用 Tauri sidecar 方式调用，修复生产环境路径问题
- 添加缓存配置（目录、大小上限、过期时间、一键清理）
- 添加日志记录到文件（按日期命名）
- 添加自动重连机制（3次重试，3秒间隔）
- 添加 5 种挂载状态的实时显示
