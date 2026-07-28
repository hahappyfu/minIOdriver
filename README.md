# MinIO Drive

MinIO 文件管理工具 - 将 MinIO bucket 映射为本地盘符

## 功能

- ✅ MinIO 连接配置管理
- ✅ Bucket 列表查看
- ✅ 一键挂载为本地盘符
- ✅ 系统托盘管理
- ✅ 开机自启

## 开发

```bash
# 安装依赖
npm install

# 开发模式
npm run tauri dev

# 构建
npm run tauri build
```

## 构建安装包

### 自动构建（推荐）

1. Fork 本仓库
2. Push 代码到 main 分支
3. GitHub Actions 会自动构建 Windows/Mac 安装包
4. 在 Releases 页面下载

### 手动构建

```bash
# Windows (需要在 Windows 上运行)
npm run tauri build

# macOS (需要在 macOS 上运行)
npm run tauri build
```

## 使用

1. 安装 [WinFsp](https://winfsp.dev/rel/)（Windows）
2. 或安装 [macFUSE](https://osxfuse.github.io/)（macOS）
3. 运行 MinIO Drive
4. 添加 MinIO 连接
5. 选择 bucket，分配盘符，点击挂载

## 技术栈

- Tauri 2.x
- React + TypeScript
- Ant Design
- rclone
