# 增量更新流程分析

**文档版本**: 1.0.0  
**分析日期**: 2025-01-18  
**源文件**: `src/data_interface/increment_manager.rs`  
**分析周期**: 完整流程端到端分析

---

## 目录

1. [执行摘要](#执行摘要)
2. [总体架构](#总体架构)
3. [完整流程分解](#完整流程分解)
4. [核心阶段详解](#核心阶段详解)
5. [关键数据结构](#关键数据结构)
6. [流程中的模块交互](#流程中的模块交互)
7. [边界条件处理](#边界条件处理)
8. [已知问题和改进空间](#已知问题和改进空间)

---

## 执行摘要

### 系统定位

增量更新流程是异地更新系统的**数据感知和处理核心**，负责：
1. 监听PDMS数据库文件的实时变化
2. 通过会话号（sesno）对比识别增量
3. 收集增量数据并更新本地数据库
4. 生成增量压缩包（CBA）供远程同步
5. 通过MQTT推送通知触发任务调度

### 关键特点

- **双重验证**: 文件会话号 + 数据库会话号对比
- **增量范围精确**: 使用会话号范围 `[db_sesno+1, file_sesno]`
- **去重机制**: 通过 `e3d_sync` 表检查文件hash避免重复
- **异步事件驱动**: 基于 `notify` 库的文件系统事件
- **全地区支持**: 通过 `location_dbs` 配置实现分地区处理

---

## 总体架构

```
┌─────────────────────────────────────────────────────────────┐
│                    增量更新系统架构                           │
└─────────────────────────────────────────────────────────────┘

输入层:
  ┌──────────────────┐
  │  PDMS数据库文件   │ (.db, 二进制格式)
  └────────┬─────────┘
           │
           │ notify 文件系统事件
           ▼
  ┌──────────────────────────────────┐
  │  PdmsWatcher (文件监听器)         │
  │  - 异步监听文件变化               │
  │  - 扫描文件头部信息               │
  │  - 维护 headers 状态map          │
  └────────┬─────────────────────────┘
           │
处理层:
           ▼
  ┌──────────────────────────────────┐
  │  AiosDBManager (增量管理器)       │
  │  - 执行增量检测                   │
  │  - 执行数据库更新                 │
  │  - 生成同步任务                   │
  └────────┬─────────────────────────┘
           │
           ├─────────────┬─────────────┬─────────────┐
           │             │             │             │
           ▼             ▼             ▼             ▼
  ┌──────────┐   ┌─────────────┐   ┌────────┐   ┌──────────┐
  │ PdmsIO   │   │ 压缩模块    │   │去重    │   │MQTT      │
  │(解析)    │   │(CBA生成)    │   │检查    │   │推送      │
  └──────────┘   └─────────────┘   └────────┘   └──────────┘
           │             │             │             │
           └─────────────┴─────────────┴─────────────┘
                        │
输出层:
                        ▼
  ┌──────────────────────────────────┐
  │  SyncControlCenter (同步中心)    │
  │  - 生成同步任务                   │
  │  - 添加到任务队列                 │
  │  - 管理任务调度                   │
  └──────────────────────────────────┘
                        │
                        ▼
  ┌──────────────────────────────────┐
  │  SurrealDB (数据库存储)          │
  │  - 更新元素数据                   │
  │  - 记录同步信息 (e3d_sync表)     │
  └──────────────────────────────────┘
```

---

## 完整流程分解

### 阶段总览

```
┌─────────────────────────────────────────────────────────────┐
│           增量更新流程 - 四个主要阶段                         │
└─────────────────────────────────────────────────────────────┘

阶段 1: 初始化 (init_watcher)
├─ 读取配置 (DbOption.toml)
├─ 扫描监听目录下所有数据库文件
├─ 执行启动后的首次增量处理
└─ 初始化监听器headers状态

        │
        ▼

阶段 2: 实时监听 (async_watch 循环)
├─ 等待文件系统事件
├─ 过滤有效的数据变化事件
├─ 扫描变化的文件头部
└─ 并发处理多个文件变化

        │
        ▼

阶段 3: 增量检测和处理 (execute_incr_update)
├─ 会话号对比检测
├─ 增量范围确定
├─ 收集增量元素
├─ 更新数据库
└─ 生成压缩包

        │
        ▼

阶段 4: 通知和调度 (enqueue_generated_sync_tasks)
├─ 去重检查
├─ MQTT推送通知
├─ 构建同步任务
└─ 添加到任务队列
```

---

## 核心阶段详解

### 1. 初始化阶段 (init_watcher)

**功能**: 系统启动时的一次性初始化扫描

**执行流程**:

```
init_watcher()
  │
  ├─ 1.1 读取配置
  │   ├─ DbOption.toml
  │   ├─ watch_dirs (监听目录列表)
  │   ├─ manual_db_nums (手动指定的数据库编号)
  │   └─ exclude_db_nums (排除的数据库编号)
  │
  ├─ 1.2 递归扫描监听目录
  │   ├─ WalkDir 遍历所有文件
  │   ├─ 按文件大小排序 (大文件优先)
  │   └─ 跳过目录项和非DB文件
  │
  ├─ 1.3 预检查和过滤
  │   ├─ 解析文件基本信息 (DbBasicInfo)
  │   ├─ 检查DB类型 (CATA/DESI/DICT/SYST/GLB/GLOB)
  │   ├─ 应用manual_db_nums过滤
  │   └─ 应用exclude_db_nums过滤
  │
  ├─ 1.4 获取文件会话号
  │   ├─ PdmsIO::new(..., true) 创建IO对象
  │   ├─ get_latest_sesno() 读取文件头部会话号
  │   └─ file_latest_sesno = sesno 值
  │
  ├─ 1.5 查询数据库会话号
  │   ├─ query_latest_sesno_by_dbnum(db_no)
  │   ├─ SELECT math::max(sesno) FROM dbnum_info_table WHERE dbnum = ?
  │   └─ db_latest_sesno = 查询结果
  │
  ├─ 1.6 会话号对比
  │   ├─ if db_latest_sesno == 0:
  │   │   └─ start_candidate = 1 (全新初始化)
  │   └─ else:
  │       └─ start_candidate = db_latest_sesno + 1
  │
  ├─ 1.7 范围检查
  │   ├─ if file_latest_sesno >= start_candidate:
  │   │   └─ 发现增量，需要处理
  │   └─ else:
  │       └─ 跳过
  │
  ├─ 1.8 查找最近会话号
  │   ├─ get_nearest_large_sesno(start_candidate)
  │   ├─ // 处理会话号不连续
  │   └─ nearest_sesno = 返回值
  │
  ├─ 1.9 构建增量范围
  │   ├─ if nearest_sesno <= file_latest_sesno:
  │   │   └─ params[path] = (basic_info, nearest_sesno..=file_latest_sesno)
  │   └─ else:
  │       └─ 跳过
  │
  ├─ 1.10 初始化headers
  │   └─ watcher.headers[path] = basic_info
  │
  ├─ 1.11 生成初始CBA包 (有MQTT特性)
  │   ├─ execute_compress(input, output.cba, temp)
  │   └─ 为后续增量下载准备
  │
  └─ 1.12 执行初始增量更新
      └─ execute_incr_update(params)
```

**关键代码位置**: Line 326-423

**输入检查**:
- ✅ DbOption.toml 配置存在
- ✅ watch_dirs 目录存在
- ✅ 数据库文件可读

**潜在风险**:
- ⚠️ 大量文件时首次启动缓慢
- ⚠️ 网络错误导致数据库查询失败会跳过文件
- ⚠️ 配置中manual_db_nums过滤可能遗漏

---

### 2. 实时监听循环 (async_watch)

**功能**: 持续监听文件系统变化，实时处理增量

**执行流程**:

```
async_watch()
  │
  ├─ 2.1 初始化监听器
  │   ├─ PdmsWatcher::async_watcher() 创建异步监听
  │   ├─ watch(watch_dir, NonRecursive) 添加监听路径
  │   ├─ 创建 assets/archives 目录
  │   └─ 创建 assets/temp 目录
  │
  └─ 2.2 进入事件循环
      │
      ├─ while let Some(res) = rx.next().await:
      │   │
      │   ├─ 2.2.1 等待事件
      │   │   └─ notify 库触发事件
      │   │
      │   ├─ 2.2.2 过滤事件类型
      │   │   ├─ if data_changed == false:
      │   │   │   └─ continue (跳过metadata变化)
      │   │   └─ 检查事件类型:
      │   │       ├─ ModifyKind::Data(_)
      │   │       ├─ ModifyKind::Any
      │   │       ├─ CreateKind::File
      │   │       └─ RemoveKind::File
      │   │
      │   ├─ 2.2.3 扫描变化文件
      │   │   ├─ PdmsWatcher::scan_db_headers(&event.paths)
      │   │   ├─ 返回 HashMap<Path, DbPageBasicInfo>
      │   │   └─ new_headers: Vec<(PathBuf, DbPageBasicInfo)>
      │   │
      │   ├─ 2.2.4 初始化工作变量
      │   │   ├─ generated_artifacts = []
      │   │   ├─ notify_file_names = []
      │   │   ├─ notify_file_hashes = []
      │   │   └─ params = {}
      │   │
      │   └─ 2.2.5 逐文件处理 (for loop)
      │       │
      │       ├─ if path 已存在 (在 headers 中):
      │       │   │
      │       │   ├─ 2.2.5a 已存在文件处理
      │       │   │   ├─ 读取新头部: new_sesno
      │       │   │   ├─ 查询数据库: query_latest_sesno_by_dbnum()
      │       │   │   ├─ 对比会话号:
      │       │   │   │   ├─ if db_latest_sesno == new_sesno:
      │       │   │   │   │   └─ continue (无增量)
      │       │   │   │   └─ else:
      │       │   │   │       └─ 有增量，继续处理
      │       │   │   ├─ 构建增量范围:
      │       │   │   │   └─ start_sesno = max(db_latest_sesno + 1, 1)
      │       │   │   └─ 添加到params:
      │       │   │       └─ params[path] = (new_header, start_sesno..=new_sesno)
      │       │   │
      │       │   └─ 处理已有文件完毕
      │       │
      │       └─ else (path 新增):
      │           │
      │           ├─ 2.2.5b 新增文件处理
      │           │   ├─ watcher.headers[path] = new_header (初始化headers)
      │           │   ├─ 提取文件名和dbno
      │           │   ├─ 检查 location_dbs 过滤:
      │           │   │   ├─ if location_dbs 配置存在:
      │           │   │   │   ├─ if !location_dbs.contains(dbno):
      │           │   │   │   │   └─ continue (不属于本地区)
      │           │   │   │   └─ else:
      │           │   │   │       └─ 继续处理
      │           │   │   └─ else:
      │           │   │       └─ 无过滤，继续处理
      │           │   │
      │           │   ├─ 生成初始CBA压缩包:
      │           │   │   ├─ execute_compress(path, archives/name.cba, temp)
      │           │   │   └─ file_hash = 返回hash值
      │           │   │
      │           │   ├─ 去重检查 (e3d_sync 表):
      │           │   │   ├─ SQL: SELECT id FROM e3d_sync
      │           │   │   │         WHERE location != ? AND ? in file_names
      │           │   │   │         AND ? in file_hashes
      │           │   │   ├─ if result.is_empty():
      │           │   │   │   ├─ notify_file_names.push(file_name)
      │           │   │   │   └─ notify_file_hashes.push(file_hash)
      │           │   │   └─ else:
      │           │   │       └─ 已同步过，跳过
      │           │   │
      │           │   ├─ 添加到同步任务 (web_server特性):
      │           │   │   └─ generated_artifacts.push(...)
      │           │   │
      │           │   └─ 处理新文件完毕
      │           │
      │           └─ 逐文件处理循环结束
      │
      │   ├─ 2.2.6 检查是否有增量需要处理
      │   │   ├─ if params.is_empty():
      │   │   │   └─ continue (跳过此次事件)
      │   │   └─ else:
      │   │       └─ 继续处理
      │   │
      │   ├─ 2.2.7 执行增量更新
      │   │   ├─ execute_incr_update(params)
      │   │   │   └─ 详见阶段3
      │   │   │
      │   │   ├─ match 返回值:
      │   │   │   ├─ Ok(true):
      │   │   │   │   ├─ 增量处理成功，继续处理
      │   │   │   │   ├─ 更新headers为新值
      │   │   │   │   ├─ 重新生成CBA压缩包
      │   │   │   │   ├─ 更新generated_artifacts
      │   │   │   │   ├─ 检查location_dbs过滤
      │   │   │   │   ├─ 去重检查 (e3d_sync)
      │   │   │   │   └─ 添加到通知列表
      │   │   │   │
      │   │   │   ├─ Ok(false):
      │   │   │   │   └─ 文件修改但无增量，记录并跳过
      │   │   │   │
      │   │   │   └─ Err(e):
      │   │   │       └─ 执行失败，记录错误
      │   │
      │   ├─ 2.2.8 MQTT推送 (有mqtt特性)
      │   │   ├─ if notify_file_names 非空:
      │   │   │   ├─ 构建 SyncE3dFileMsg
      │   │   │   ├─ INSERT INTO e3d_sync
      │   │   │   └─ mqtt_client.publish("Sync/E3d", payload)
      │   │   └─ else:
      │   │       └─ 无需推送
      │   │
      │   ├─ 2.2.9 加入同步任务 (web_server特性)
      │   │   ├─ enqueue_generated_sync_tasks(generated_artifacts)
      │   │   │   └─ 详见阶段4
      │   │   └─ 任务添加完毕
      │   │
      │   └─ 2.2.10 事件处理完毕
      │       └─ 继续等待下一个事件
      │
      └─ 事件循环（持续运行，直到错误或异常退出）
```

**关键代码位置**: Line 470-815

**事件处理流程中的关键决策点**:

| 检查点 | 条件 | 动作 | 影响 |
|-------|------|------|------|
| 事件类型 | data_changed | 否 → 跳过 | 减少无效处理 |
| 文件存在 | path in headers | 新增 → 初始化 | 处理新文件 |
| 会话号 | file_sesno > db_sesno | 否 → 跳过 | 检测增量 |
| 地区过滤 | location_dbs | 否 → 跳过 | 地区隔离 |
| 去重检查 | e3d_sync 查询 | 重复 → 跳过 | 避免重复 |

---

### 3. 增量检测和处理 (execute_incr_update)

**功能**: 核心的增量数据处理逻辑

**执行流程**:

```
execute_incr_update(increment_ranges_map)
  │
  ├─ 3.1 参数检查
  │   ├─ if increment_ranges_map.is_empty():
  │   │   └─ return Ok(false)
  │   └─ else:
  │       └─ 继续处理
  │
  ├─ 3.2 初始化处理标志
  │   └─ has_updates = false
  │
  └─ 3.3 遍历增量范围 (for loop)
      │
      ├─ for (path, (basic_info, sesno_range)) in increment_ranges_map:
      │   │
      │   ├─ 3.3.1 打开数据库文件
      │   │   ├─ PdmsIO::new("", path, true)
      │   │   ├─ io.open() 打开文件
      │   │   └─ end_sesno = sesno_range.end()
      │   │
      │   ├─ 3.3.2 收集增量元素
      │   │   ├─ io.collect_increment_eles(Some(sesno_range))
      │   │   ├─ 返回 Vec<(RefNo, EleOperation, ElementData)>
      │   │   └─ range_update_eles = 收集结果
      │   │
      │   ├─ 3.3.3 检查结果
      │   │   ├─ if range_update_eles.is_empty():
      │   │   │   └─ continue (无增量元素)
      │   │   └─ else:
      │   │       └─ 继续处理
      │   │
      │   ├─ 3.3.4 更新数据库
      │   │   ├─ io.update_elements_to_database(range_update_eles, true).await
      │   │   ├─ 批量INSERT/UPDATE元素
      │   │   ├─ 更新SurrealDB中的元素记录
      │   │   └─ has_updates = true
      │   │
      │   ├─ 3.3.5 更新会话号书签
      │   │   ├─ UPDATE db_file_info:{file_name} SET sesno={end_sesno}
      │   │   ├─ SurrealDB 中的记录
      │   │   └─ 保存处理进度
      │   │
      │   └─ 3.3.6 处理下一个文件
      │
      └─ 3.4 返回处理结果
          └─ return Ok(has_updates)
```

**关键代码位置**: Line 214-280

**数据收集流程详解**:

```
collect_increment_eles() 内部:
  │
  ├─ 1. 打开PDMS文件的内存映射
  ├─ 2. 解析页面结构
  ├─ 3. 遍历sesno_range范围
  │   └─ for sesno in start_sesno..=end_sesno:
  │       ├─ 查找sesno对应的页面数据
  │       ├─ 解析元素操作记录 (EleOperation)
  │       └─ 收集元素数据
  ├─ 4. 去重和合并
  └─ 5. 返回元素列表
```

**元素操作类型**:
- `EleOperation::Add` - 新增元素
- `EleOperation::Modified` - 修改元素
- `EleOperation::Deleted` - 删除元素

**更新到数据库流程**:
```
update_elements_to_database():
  │
  ├─ 1. 分批处理 (JSON_CHUNK_COUNT = 200)
  ├─ 2. 为每个元素生成INSERT/UPDATE语句
  ├─ 3. 创建SurrealDB事务
  ├─ 4. 执行批量操作
  └─ 5. 提交事务
```

---

### 4. 通知和调度阶段 (enqueue_generated_sync_tasks)

**功能**: 将已处理的增量转换为同步任务

**执行流程**:

```
enqueue_generated_sync_tasks(artifacts)
  │
  ├─ 4.1 参数检查
  │   ├─ if artifacts.is_empty():
  │   │   └─ return (无任务)
  │   └─ else:
  │       └─ 继续处理
  │
  ├─ 4.2 获取环境信息
  │   ├─ REMOTE_RUNTIME.read().await
  │   ├─ 读取 env_id
  │   └─ 失败则返回
  │
  ├─ 4.3 查询远程同步配置
  │   ├─ spawn_blocking 执行数据库查询
  │   ├─ 打开 SQLite 连接
  │   ├─ SELECT name FROM remote_sync_envs WHERE id = ?
  │   ├─ SELECT id, name FROM remote_sync_sites WHERE env_id = ?
  │   └─ 返回 (env_name, site_entries)
  │
  ├─ 4.4 构建目标站点列表
  │   ├─ if site_entries.is_empty():
  │   │   └─ targets = [(None, None)] (通用目标)
  │   └─ else:
  │       └─ targets = site_entries (逐站点处理)
  │
  ├─ 4.5 获取源环境信息
  │   ├─ source_env = DbOption.location
  │   └─ 记录本地环境
  │
  └─ 4.6 遍历任务和目标 (double loop)
      │
      ├─ for artifact in artifacts:
      │   │
      │   ├─ for (site_id, site_name) in targets:
      │   │   │
      │   │   ├─ 4.6.1 检查路径有效性
      │   │   │   ├─ artifact.path.to_str() -> Some(path_str)
      │   │   │   └─ 无效则跳过
      │   │   │
      │   │   ├─ 4.6.2 构建同步任务参数
      │   │   │   └─ NewSyncTaskParams {
      │   │   │       file_path: path_str,
      │   │   │       file_size: artifact.file_size,
      │   │   │       priority: 5,
      │   │   │       record_count: artifact.record_count,
      │   │   │       file_name: artifact.file_name,
      │   │   │       file_hash: artifact.file_hash,
      │   │   │       env_id: env_id,
      │   │   │       source_env: source_env,
      │   │   │       target_site: site_id,
      │   │   │       direction: "UPLOAD",
      │   │   │       notes: format!("自动同步 - {}", site_name)
      │   │   │     }
      │   │   │
      │   │   ├─ 4.6.3 添加到控制中心
      │   │   │   ├─ SYNC_CONTROL_CENTER.write().await
      │   │   │   └─ center.add_task(params)
      │   │   │
      │   │   └─ 4.6.4 继续下一个目标
      │   │
      │   └─ 4.6.5 继续下一个任务
      │
      └─ 4.7 所有任务添加完毕，返回
```

**关键代码位置**: Line 105-191

**任务构建的关键信息**:

| 字段 | 来源 | 说明 |
|------|------|------|
| file_path | artifact.path | 压缩包的本地路径 |
| file_size | artifact.file_size | 压缩包大小 |
| priority | 固定值 5 | 优先级中等 |
| record_count | artifact.record_count | 增量元素数量 |
| file_hash | artifact.file_hash | 校验和 |
| direction | 固定值 "UPLOAD" | 上传到远程 |

---

## 关键数据结构

### IncrementInfo

```rust
pub struct IncrementInfo {
    pub refno: RefU64,           // 元素引用号
    pub db_no: i32,              // 数据库编号
    pub attr: NamedAttrMap,      // 属性映射
    pub children: RefU64Vec,     // 子元素列表
    pub operation: EleOperation, // 操作类型
}
```

**操作类型**:
```rust
pub enum EleOperation {
    Add,      // 新增
    Modified, // 修改
    Deleted,  // 删除
}
```

### DbPageBasicInfo

```rust
pub struct DbPageBasicInfo {
    pub pdms_header: PdmsHeader,           // PDMS文件头
    pub latest_ses_data: SessionData,      // 最新会话号信息
}

pub struct SessionData {
    pub sesno: i32,  // 会话号
    // ...其他字段
}
```

### GeneratedSyncArtifact

```rust
struct GeneratedSyncArtifact {
    path: PathBuf,              // CBA压缩包路径
    file_name: String,          // 文件名 (带.cba扩展)
    file_size: u64,             // 文件大小
    file_hash: Option<String>,  // SHA256校验和
    record_count: Option<u64>,  // 增量元素数量
}
```

---

## 流程中的模块交互

### 跨模块调用关系

```
increment_manager.rs
├─ 调用 PdmsWatcher
│   ├─ scan_db_headers(&event.paths)
│   └─ async_watcher()
│
├─ 调用 PdmsIO
│   ├─ open()
│   ├─ get_latest_sesno()
│   ├─ get_nearest_large_sesno(sesno)
│   ├─ collect_increment_eles(range)
│   └─ update_elements_to_database(elements, async)
│
├─ 调用 execute_compress (CompressOptions)
│   ├─ 生成 CBA 压缩包
│   └─ 返回 SHA256 hash
│
├─ 调用 SurrealDB (SUL_DB)
│   ├─ query_latest_sesno_by_dbnum(dbnum)
│   ├─ query_latest_sesno_by_file_name(file_name)
│   ├─ UPDATE db_file_info:{}
│   ├─ SELECT FROM e3d_sync (去重)
│   └─ INSERT INTO e3d_sync
│
├─ 调用 MQTT
│   ├─ publish("Sync/E3d", QoS::ExactlyOnce)
│   └─ SyncE3dFileMsg payload
│
└─ 调用 SyncControlCenter (web_server特性)
    └─ add_task(NewSyncTaskParams)
```

### 数据流向图

```
文件系统
   │
   │ notify 事件
   ▼
PdmsWatcher
   │
   │ scan_db_headers()
   │ 返回 HashMap<Path, DbPageBasicInfo>
   ▼
increment_manager.rs (async_watch 循环)
   │
   ├─ 对比会话号
   │   ├─ query_latest_sesno_by_dbnum() ─┐
   │   │ (查询 SurrealDB)                 │
   │   └─ 获得 db_sesno                   │
   │                                       │ SurrealDB
   ├─ 构建增量范围                         │
   │   └─ [db_sesno+1, file_sesno]        │
   │                                       │
   ├─ execute_incr_update()                │
   │   │                                   │
   │   ├─ PdmsIO::collect_increment_eles() │
   │   │   └─ 返回增量元素列表             │
   │   │                                   │
   │   └─ io.update_elements_to_database()─┼──▶ SurrealDB (插入/更新元素)
   │       └─ 返回 Ok/Err                  │
   │                                       │
   ├─ execute_compress()                   ├──▶ 文件系统 (生成 assets/archives/xxx.cba)
   │   └─ 返回 SHA256 hash                 │
   │                                       │
   ├─ 去重检查                             │
   │   └─ SELECT FROM e3d_sync ─────────────────┐
   │       (检查是否已同步过)                    │
   │                                       │    │
   ├─ MQTT 推送 ─────────────────────────────┼──▶ MQTT Broker
   │   └─ publish("Sync/E3d", payload)     │    │
   │       (通知远程开始同步)              │    │
   │                                       │    │
   ├─ 记录同步信息 ──────────────────────────┼─────▶ SurrealDB (e3d_sync表)
   │   └─ INSERT INTO e3d_sync               │
   │       (避免重复推送)                    │
   │                                       │
   └─ enqueue_generated_sync_tasks()      │
       │                                   │
       ├─ 查询同步配置 ─────────────────────────┼──▶ SQLite (remote_sync_envs, remote_sync_sites)
       │                                   │
       └─ add_task(SyncControlCenter) ──────────────▶ SyncControlCenter (任务队列)
```

---

## 边界条件处理

### 1. 会话号相关

| 场景 | 处理方式 | 结果 |
|------|--------|------|
| db_sesno == 0 | start_candidate = 1 | 首次初始化，扫描全部 |
| file_sesno == db_sesno | 跳过 | 无增量 |
| file_sesno < db_sesno | 跳过 | 异常情况（数据库领先） |
| 会话号不连续 | get_nearest_large_sesno() | 找到最近有效sesno |
| sesno 回滚 | 数据库记录的历史 | （目前无特殊处理） |

### 2. 文件相关

| 场景 | 处理方式 | 结果 |
|------|--------|------|
| 新文件出现 | 初始化headers，生成CBA | 纳入监听 |
| 文件删除 | notify 事件 | （目前无删除处理） |
| 文件不可读 | 异常捕获，记录错误 | 跳过此文件 |
| 文件过大 | 分块处理 | 增量分多批 |

### 3. 地区隔离

| 配置 | 影响 | 作用 |
|------|------|------|
| location_dbs 为空 | 无过滤 | 处理所有数据库 |
| location_dbs = [1001, 1002] | 仅处理这些dbnum | 实现多地区分工 |
| dbnum 不在列表中 | 跳过处理 | 避免跨地区干扰 |

### 4. 去重机制

| 场景 | SQL查询 | 结果 |
|------|--------|------|
| 首次同步 | 查询为空 | 添加到通知列表 |
| 重复同步（hash相同） | 查询有结果 | 跳过，避免重复 |
| 跨地区同步 | location != ? | 允许地区间同步 |

---

## 已知问题和改进空间

### P0 - 关键问题

#### 1. 并发安全问题

**位置**: Line 578-591, 663-666

```rust
// 问题代码
if let Some(mut old) = self.watcher.headers.get_mut(path) {
    // ... 长时间持有互斥锁
    *old.value_mut() = new_header.clone(); // 更新headers
}
```

**风险**:
- ⚠️ 获取 `get_mut()` 后长时间操作（查询DB、压缩、MQTT）
- ⚠️ 其他线程无法获取 headers，导致后续事件处理延迟
- ⚠️ 极端情况下可能导致死锁

**改进建议**:
- ✅ 先读取信息，然后释放锁
- ✅ 在锁外进行I/O操作
- ✅ 使用异步任务分离长时间操作

#### 2. 错误处理不完整

**位置**: Line 381, 556, 595 等多处

```rust
// 问题：多处使用 continue 跳过错误
let Ok(db_latest_sesno) = Self::query_latest_sesno_by_dbnum(db_no).await else {
    continue; // 跳过文件，无错误记录
};
```

**风险**:
- ⚠️ 数据库连接失败时无重试
- ⚠️ 压缩失败时直接跳过，无回滚
- ⚠️ MQTT推送失败时无重试机制
- ⚠️ 无错误日志持久化

**改进建议**:
- ✅ 实现失败重试队列
- ✅ 记录错误日志到数据库
- ✅ 实现死信队列处理失败任务

#### 3. 性能瓶颈

**位置**: Line 383, 581 (数据库查询)

```rust
// 每次文件变化都查询数据库
let db_latest_sesno = 
    Self::query_latest_sesno_by_dbnum(db_num as _).await?;
```

**风险**:
- ⚠️ 大量文件同时变化时数据库查询暴增
- ⚠️ 无缓存机制，重复查询相同dbnum
- ⚠️ 可能导致数据库连接池耗尽

**改进建议**:
- ✅ 实现本地sesno缓存（带TTL）
- ✅ 批量查询而非逐个查询
- ✅ 定期后台更新缓存

### P1 - 主要问题

#### 4. 会话号回滚处理缺失

**位置**: Line 580

```rust
// 无法处理会话号回滚情况
if db_latest_sesno as i32 == new_sesno {
    continue;
}
```

**风险**:
- ⚠️ 如果数据库会话号倒退（数据回滚），无法检测
- ⚠️ 可能导致增量数据重复或不一致

**改进建议**:
- ✅ 记录会话号历史
- ✅ 检测异常回滚
- ✅ 触发告警或手动干预流程

#### 5. 新文件初始化逻辑复杂

**位置**: Line 589-689

```rust
// 新文件处理代码过长，混合了多个关注点
// - 初始化headers
// - 生成CBA
// - 去重检查
// - MQTT推送
```

**改进建议**:
- ✅ 拆分为独立函数 `handle_new_file()`
- ✅ 提高代码可读性
- ✅ 便于单元测试

#### 6. 缺少监控指标

**风险**:
- ⚠️ 无法追踪增量处理性能
- ⚠️ 无法监测错误率和重试次数
- ⚠️ 难以定位性能瓶颈

**改进建议**:
- ✅ 添加计时器（整体耗时、各阶段耗时）
- ✅ 计数器（处理文件数、元素数、错误数）
- ✅ 性能指标上报

### P2 - 可选改进

#### 7. 配置灵活性

**改进建议**:
- 支持每个数据库的独立处理策略
- 支持临时暂停/启用特定数据库监听
- 支持动态调整优先级

#### 8. 测试覆盖

**改进建议**:
- 单元测试：会话号对比逻辑
- 集成测试：完整增量处理流程
- 压力测试：大量文件同时变化

---

## 总结

### 系统强点

✅ **准确的增量检测**: 基于文件会话号 + 数据库会话号的双重验证  
✅ **完整的信息流**: 从文件检测到任务调度的全链路  
✅ **地区隔离支持**: 通过 `location_dbs` 实现多地区分工  
✅ **去重机制**: 避免重复推送相同增量  
✅ **异步事件驱动**: 基于 notify 库的高效文件监听  

### 需要改进的地方

⚠️ **并发安全**: 长时间持有锁导致的响应延迟  
⚠️ **错误恢复**: 缺乏重试和失败恢复机制  
⚠️ **性能优化**: 数据库查询无缓存，高并发下压力大  
⚠️ **代码复杂度**: 大函数混合多个关注点  
⚠️ **可观测性**: 缺乏性能监控和错误追踪  

### 建议的优化顺序

1. **短期** (1-2周): 并发安全优化 + 错误处理完善
2. **中期** (2-4周): 缓存机制 + 代码拆分
3. **长期** (1-2个月): 监控系统 + 测试框架

