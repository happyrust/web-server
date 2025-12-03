# 增量更新前端 (Vue 3 + Vite)

这是 AIOS Database 增量更新界面的 Vue 3 重构版本，使用 Vite 构建工具打包成单文件输出。

## 📦 项目结构

```
frontend/
├── src/
│   ├── components/          # Vue 组件
│   │   ├── SiteCard.vue    # 站点卡片组件
│   │   └── TaskQueue.vue   # 任务队列组件
│   ├── composables/        # 组合式函数（可复用逻辑）
│   │   ├── useApi.js       # API 调用
│   │   ├── useTheme.js     # 主题切换
│   │   ├── useNotification.js  # 通知系统
│   │   └── useFormatters.js    # 格式化工具
│   ├── App.vue             # 主应用组件
│   ├── main.js             # 入口文件
│   └── style.css           # 全局样式
├── index.html              # 开发模式 HTML
├── package.json            # 依赖配置
├── vite.config.js          # Vite 构建配置
├── tailwind.config.js      # Tailwind CSS 配置
└── postcss.config.js       # PostCSS 配置
```

## 🚀 快速开始

### 1. 安装依赖

```bash
cd frontend
npm install
```

**需要安装的依赖**：
- `vue@^3.4.0` - Vue 3 核心
- `@vitejs/plugin-vue@^5.0.0` - Vite 的 Vue 插件
- `vite@^5.0.0` - 构建工具
- `tailwindcss@^3.4.0` + `daisyui` - CSS 框架
- `echarts@^5.4.3` - 图表库（外部依赖，不打包）

### 2. 开发模式

启动开发服务器（热重载 + 代理后端 API）：

```bash
npm run dev
```

访问 `http://localhost:3000` 查看效果。API 请求会代理到 `http://localhost:8080`。

### 3. 生产构建

构建为单文件输出：

```bash
npm run build
```

**构建产物**（在 `dist/` 目录）：
- `incremental_update.js` - 所有 Vue 组件和逻辑（压缩后约 100-150KB）
- `incremental_update.css` - 所有样式（Tailwind + DaisyUI）

### 4. 部署到 Rust 后端

自动复制构建产物到后端静态资源目录：

```bash
npm run deploy
```

这会执行：
1. `npm run build` - 构建生产版本
2. 复制 `dist/incremental_update.js` 到 `../src/web_server/static/`
3. 复制 `dist/incremental_update.css` 到 `../src/web_server/static/`

**注意**：Windows 使用 `xcopy`，Linux/Mac 需要修改 `package.json` 中的 `deploy` 脚本为 `cp`。

## 🔧 Vite 配置详解

### 单文件输出配置

```javascript
// vite.config.js
build: {
  lib: {
    entry: 'src/main.js',
    name: 'IncrementalApp',
    formats: ['iife'],              // 立即执行函数，浏览器直接运行
    fileName: () => 'incremental_update.js'
  },
  rollupOptions: {
    output: {
      inlineDynamicImports: true,   // 内联所有动态导入，打包成单文件
    },
    external: ['echarts'],          // ECharts 不打包，使用 CDN
    globals: { echarts: 'echarts' }
  }
}
```

### 开发代理配置

```javascript
// vite.config.js
server: {
  port: 3000,
  proxy: {
    '/api': {
      target: 'http://localhost:8080',  // 代理到 Rust 后端
      changeOrigin: true
    }
  }
}
```

## 🎨 技术栈

| 技术 | 版本 | 用途 |
|------|------|------|
| **Vue 3** | 3.4+ | 组件框架（Composition API） |
| **Vite** | 5.0+ | 构建工具（热重载、打包） |
| **Tailwind CSS** | 3.4+ | 原子化 CSS 框架 |
| **DaisyUI** | 4.6+ | Tailwind 组件库 |
| **ECharts** | 5.4+ | 图表库（CDN，不打包） |
| **Font Awesome** | 6.4+ | 图标库（CDN） |

## 📝 组件说明

### `SiteCard.vue` - 站点卡片组件

**Props**:
- `site` (Object) - 站点数据对象

**Events**:
- `@detect` - 触发检测变更
- `@sync` - 触发同步
- `@abort` - 中止操作
- `@detail` - 查看详情

**功能**:
- 显示站点状态（空闲、扫描中、同步中等）
- 显示统计指标（待同步、已同步、增量大小）
- 显示变更文件列表
- 同步进度条（如果正在同步）

### `TaskQueue.vue` - 任务队列组件

**Props**:
- `tasks` (Array) - 任务数组

**功能**:
- 显示待处理任务列表
- 任务状态标签（Pending、Running、Completed、Failed）
- 任务详细信息（文件大小、优先级、创建时间）

## 🔌 组合式函数 (Composables)

### `useApi.js` - API 调用

```javascript
import { useApi } from './composables/useApi';

const { loadSites, triggerDetection, triggerSync } = useApi();

// 加载站点
const data = await loadSites();

// 触发检测
await triggerDetection('site-123');
```

### `useTheme.js` - 主题切换

```javascript
import { useTheme } from './composables/useTheme';

const { isDarkMode, toggleTheme } = useTheme();

// 切换主题
toggleTheme();
```

### `useNotification.js` - 通知系统

```javascript
import { useNotification } from './composables/useNotification';

const { success, error, info, warning } = useNotification();

// 显示通知
success('操作成功！');
error('操作失败！');
```

### `useFormatters.js` - 格式化工具

```javascript
import { useFormatters } from './composables/useFormatters';

const { formatTime, formatSize, getStatusClass } = useFormatters();

formatTime(new Date());       // "2024-01-15 14:30"
formatSize(1024 * 1024);      // "1 MB"
getStatusClass('Completed');  // "status-completed"
```

## 🔄 与原生 JS 版本对比

| 特性 | 原生 JS | Vue 3 版本 |
|------|---------|-----------|
| **代码行数** | ~870 行 | ~600 行（组件化拆分） |
| **状态管理** | 手动 DOM 操作 | 响应式数据绑定 |
| **组件复用** | 字符串拼接 | Vue 组件 |
| **类型安全** | 无 | 可选 TypeScript |
| **开发体验** | 基础文本编辑 | 热重载 + 组件预览 |
| **文件大小** | ~50KB | ~150KB（含 Vue 运行时） |
| **维护性** | ⭐⭐ | ⭐⭐⭐⭐⭐ |

## 🛠️ 集成到 Rust 后端

### 步骤 1：构建前端

```bash
cd frontend
npm run build
```

### 步骤 2：更新 HTML 模板

修改 `src/web_server/templates/incremental_update.html`：

```html
<!DOCTYPE html>
<html lang="zh-CN" data-theme="light">
<head>
  <meta charset="UTF-8">
  <title>增量更新检测</title>

  <!-- Font Awesome -->
  <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.4.0/css/all.min.css">

  <!-- ECharts -->
  <script src="https://cdn.jsdelivr.net/npm/echarts@5.4.3/dist/echarts.min.js"></script>

  <!-- Vue 构建产物 -->
  <link rel="stylesheet" href="/static/incremental_update.css">
</head>
<body>
  <div id="app"></div>
  <script src="/static/incremental_update.js"></script>
</body>
</html>
```

### 步骤 3：复制静态文件

```bash
# Windows
npm run deploy

# 或手动复制
xcopy /Y dist\*.js ..\src\web_server\static\
xcopy /Y dist\*.css ..\src\web_server\static\
```

### 步骤 4：启动 Rust 后端

```bash
cd ..
cargo run --bin web_server --features web_server
```

访问 `http://localhost:8080/incremental` 查看效果！

## 🐛 故障排查

### 问题 1：构建失败 - `npm install` 报错

**解决**：
```bash
# 清除缓存重新安装
rm -rf node_modules package-lock.json
npm install
```

### 问题 2：开发模式下 API 404

**检查**：
- Rust 后端是否在 `http://localhost:8080` 运行
- Vite 代理配置是否正确（`vite.config.js` 中的 `proxy` 配置）

### 问题 3：生产构建后样式丢失

**检查**：
- 是否同时引入了 `incremental_update.css`
- HTML `<head>` 中是否有 `<link rel="stylesheet" href="/static/incremental_update.css">`

### 问题 4：ECharts 图表不显示

**检查**：
- CDN 是否加载成功（查看浏览器控制台）
- `vite.config.js` 中 `external: ['echarts']` 是否配置
- HTML 中是否引入 ECharts CDN

## 📚 扩展开发

### 添加新组件

1. 在 `src/components/` 创建新组件：

```vue
<!-- src/components/MyComponent.vue -->
<template>
  <div>{{ message }}</div>
</template>

<script setup>
import { ref } from 'vue';

const message = ref('Hello Vue!');
</script>
```

2. 在 `App.vue` 中引入：

```javascript
import MyComponent from './components/MyComponent.vue';
```

### 添加新的 API 方法

在 `src/composables/useApi.js` 中添加：

```javascript
async function myNewApi() {
  return fetchData('/api/my-endpoint', { method: 'POST' });
}

return {
  // ...其他方法
  myNewApi
};
```

## 🎯 下一步计划

- [ ] 添加同步历史组件 (`SyncHistory.vue`)
- [ ] 添加日志查看器组件 (`LogViewer.vue`)
- [ ] 添加 ECharts 图表组件 (`Charts/`)
- [ ] 添加 TypeScript 支持
- [ ] 添加单元测试 (Vitest)
- [ ] 优化打包体积（Tree Shaking）

## 📄 License

与主项目保持一致。
