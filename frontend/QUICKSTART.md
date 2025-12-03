# 快速开始指南

## ⚡ 5 分钟快速上手

### 第一步：安装依赖

```bash
cd frontend
npm install
```

如果 `npm install` 太慢，可以使用国内镜像：

```bash
npm install --registry=https://registry.npmmirror.com
```

### 第二步：启动开发服务器

```bash
npm run dev
```

浏览器自动打开 `http://localhost:3000`。

**注意**：确保 Rust 后端在 `localhost:8080` 运行，否则 API 请求会失败。

### 第三步：构建生产版本

```bash
npm run build
```

构建完成后，在 `dist/` 目录会生成：
- `incremental_update.js` - Vue 应用程序（压缩后）
- `incremental_update.css` - 样式文件

### 第四步：部署到后端

#### 方法 1：自动部署（推荐）

```bash
npm run deploy
```

这会自动将构建产物复制到 `../src/web_server/static/` 目录。

#### 方法 2：手动部署

**Windows**:
```cmd
xcopy /Y dist\incremental_update.js ..\src\web_server\static\
xcopy /Y dist\incremental_update.css ..\src\web_server\static\
```

**Linux/Mac**:
```bash
cp dist/incremental_update.js ../src/web_server/static/
cp dist/incremental_update.css ../src/web_server/static/
```

### 第五步：更新 HTML 模板

修改 `src/web_server/templates/incremental_update.html`：

```html
<!DOCTYPE html>
<html lang="zh-CN" data-theme="light">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>增量更新检测 - AIOS Database</title>

  <!-- Font Awesome -->
  <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">

  <!-- ECharts (外部依赖) -->
  <script src="https://cdn.jsdelivr.net/npm/echarts@5.4.3/dist/echarts.min.js"></script>

  <!-- Vue 构建产物 CSS -->
  <link rel="stylesheet" href="/static/incremental_update.css">
</head>
<body>
  <!-- Vue 挂载点 -->
  <div id="app"></div>

  <!-- Vue 构建产物 JS -->
  <script src="/static/incremental_update.js"></script>
</body>
</html>
```

### 第六步：启动 Rust 后端

```bash
cd ..
cargo run --bin web_server --features web_server
```

访问 `http://localhost:8080/incremental` 查看效果！

## 🔄 开发工作流

### 日常开发

1. **启动前后端**：

```bash
# 终端 1 - Rust 后端
cargo run --bin web_server --features web_server

# 终端 2 - Vue 前端（开发模式）
cd frontend
npm run dev
```

2. **访问**：
   - 开发模式：`http://localhost:3000` （热重载，自动刷新）
   - 生产模式：`http://localhost:8080/incremental` （需要先构建）

3. **修改代码**：
   - 编辑 `src/components/*.vue` 或 `src/App.vue`
   - 浏览器自动刷新（热重载）

4. **查看效果**：
   - 保存文件后浏览器自动更新
   - 无需手动刷新

### 发布流程

```bash
# 1. 构建前端
cd frontend
npm run build

# 2. 部署到后端
npm run deploy

# 3. 构建 Rust
cd ..
cargo build --release

# 4. 运行
cargo run --release --bin web_server --features web_server
```

## 📦 npm 脚本说明

| 命令 | 说明 |
|------|------|
| `npm run dev` | 启动开发服务器（热重载） |
| `npm run build` | 构建生产版本 |
| `npm run preview` | 预览构建产物 |
| `npm run deploy` | 构建并复制到后端静态目录 |

## 🐛 常见问题

### Q1: `npm install` 失败

**A**: 尝试清除缓存：

```bash
rm -rf node_modules package-lock.json
npm cache clean --force
npm install
```

### Q2: 开发模式下 API 404

**A**: 确保 Rust 后端在运行：

```bash
# 检查端口是否被占用
netstat -ano | findstr :8080    # Windows
lsof -i :8080                   # Linux/Mac

# 启动后端
cargo run --bin web_server --features web_server
```

### Q3: 构建后样式丢失

**A**: 检查 HTML 是否引入 CSS：

```html
<link rel="stylesheet" href="/static/incremental_update.css">
```

### Q4: 图表不显示

**A**: 检查 ECharts CDN 是否加载：

```html
<script src="https://cdn.jsdelivr.net/npm/echarts@5.4.3/dist/echarts.min.js"></script>
```

## 🎯 下一步

- 阅读完整的 [README.md](README.md)
- 查看 [Vite 配置说明](README.md#-vite-配置详解)
- 学习 [组合式函数](README.md#-组合式函数-composables)
- 添加自定义组件

## 💡 提示

- **开发时使用 `npm run dev`**，修改代码自动刷新
- **部署前使用 `npm run build`**，生成压缩版本
- **使用 `npm run deploy`** 自动部署到后端
- 修改 `vite.config.js` 可以调整构建配置
- 修改 `tailwind.config.js` 可以自定义主题

祝开发愉快！🚀
