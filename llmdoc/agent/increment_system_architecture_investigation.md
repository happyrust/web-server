# 增量更新系统架构深度调查报告

**调查时间**: 2025-11-20
**报告版本**: 1.0
**调查范围**: 增量更新系统完整架构、状态管理、失败恢复、Web监控接口

---

## Code Sections (The Evidence)

### 核心数据结构

- `src/data_interface/failed_task_queue.rs:36-75` (FailedTaskType): 定义4种失败任务类型 - DatabaseQuery(数据库查询失败)、Compression(CBA压缩失败)、IncrementUpdate(增量更新失败)、MqttPublish(MQTT推送失败)

- `src/data_interface/failed_task_queue.rs:100-139` (FailedTask): 失败任务完整记录结构,包含id(UUID)、task_type、error、retry_count(重试次数)、max_retries(最大重试次数,默认5)、first_failed_at/last_retry_at/next_retry_at(时间戳)、priority(优先级1-10)、metadata(JSON元数据)

- `src/data_interface/failed_task_queue.rs:212-236` (FailedTaskQueue): 失败任务队列管理器,使用Arc<RwLock<Vec<FailedTask>>>存储内存队列,persist_path指向持久化文件(assets/failed_tasks.json)

- `src/data_interface/failed_task_queue.rs:399-409` (TaskQueueStats): 队列统计信息 - total(总任务数)、pending(待重试)、waiting(等待中)、exhausted(已耗尽)

- `src/data_interface/increment_manager.rs:50-97` (IncrementInfo): 增量更新信息结构,包含refno(元素引用号)、db_no(数据库编号)、attr(属性映射)、children(子元素列表)、operation(操作类型:Add/Modified/Deleted)

- `src/data_interface/increment_manager.rs:100-106` (IncrementUpdateStats): 增量更新统计 - total_added、total_modified、total_deleted

- `src/data_interface/increment_manager.rs:110-127` (GeneratedSyncArtifact): 生成的同步产物信息,包含path(CBA文件路径)、file_name、file_size、file_hash(SHA256)、record_count(元素数量)、db_num、old_sesno/new_sesno、session_range、generated_at、is_full_sync(是否全量同步)、统计字段(total_added/modified/deleted)

### 失败任务队列核心API

- `src/data_interface/failed_task_queue.rs:142-158` (FailedTask::new): 创建新失败任务,自动生成UUID、设置初始重试时间(1分钟后)、默认优先级5、最大重试5次

- `src/data_interface/failed_task_queue.rs:184-192` (schedule_next_retry): 计算下次重试时间,使用指数退避策略 - delay_secs = 60 * (2^retry_count),即1/2/4/8/16分钟

- `src/data_interface/failed_task_queue.rs:194-198` (should_retry): 判断是否应该重试,条件为retry_count < max_retries 且 当前时间 >= next_retry_at

- `src/data_interface/failed_task_queue.rs:200-203` (is_exhausted): 判断是否达到最大重试次数

- `src/data_interface/failed_task_queue.rs:238-249` (push): 添加失败任务到队列,写锁保护,异步持久化到JSON文件

- `src/data_interface/failed_task_queue.rs:269-277` (get_pending_tasks): 获取所有应该重试的任务(满足should_retry条件)

- `src/data_interface/failed_task_queue.rs:285-300` (get_stats): 返回队列统计信息,读锁保护

- `src/data_interface/failed_task_queue.rs:302-318` (remove): 移除任务(重试成功后),写锁保护,异步持久化

- `src/data_interface/failed_task_queue.rs:320-333` (update): 更新任务状态(重试失败后),写锁保护,异步持久化

- `src/data_interface/failed_task_queue.rs:335-339` (get_exhausted_tasks): 获取所有已耗尽的任务(用于告警)

- `src/data_interface/failed_task_queue.rs:341-360` (cleanup_exhausted): 手动清理已耗尽的任务,返回清理数量

- `src/data_interface/failed_task_queue.rs:362-383` (persist): 持久化到磁盘,使用原子写入(先写临时文件.tmp,再重命名),确保目录存在

- `src/data_interface/failed_task_queue.rs:385-395` (load_from_disk): 从磁盘加载失败任务,JSON反序列化

### 增量更新主流程

- `src/data_interface/increment_manager.rs:878` (async_watch): 异步文件监听循环,使用notify库监听PDMS数据库文件变化,处理ModifyKind::Data、CreateKind::File、RemoveKind::File事件

- `src/data_interface/increment_manager.rs:647` (execute_incr_update): 执行增量更新核心逻辑,遍历增量范围映射,调用PdmsIO::collect_increment_eles收集增量元素,调用update_elements_to_database批量更新到SurrealDB

- `pdms_io/io.rs` (collect_increment_eles): 从PDMS文件收集指定sesno范围内的增量元素,返回Vec<(RefNo, EleOperation, ElementData)>

- `pdms_io/io.rs` (update_elements_to_database): 批量更新元素到SurrealDB,分批处理(JSON_CHUNK_COUNT=200),生成INSERT/UPDATE/DELETE语句并执行事务

### 增量检测与会话号管理

- `docs/INCREMENT_DETECTION_FLOWCHART.md:280-295` (query_latest_sesno_by_dbnum): 查询数据库最新会话号,SQL: SELECT math::max(array::flatten([SELECT VALUE sesno FROM dbnum_info_table WHERE dbnum = ?]))

- `docs/INCREMENT_DETECTION_FLOWCHART.md:496-516` (会话号对比逻辑): if file_sesno > db_sesno 则有增量,range = (db_sesno + 1)..=file_sesno

- `docs/INCREMENT_DETECTION_FLOWCHART.md:199-208` (get_nearest_large_sesno): 处理会话号不连续情况,查找最近的大于start_sesno的有效会话号

### 错误处理集成点

- `src/data_interface/increment_manager.rs:722-735` (数据库查询失败处理): 查询sesno失败时创建FailedTask(DatabaseQuery类型),附加metadata包含file_path、old_sesno、new_sesno,推入failed_queue

- `src/data_interface/increment_manager.rs:819-834` (新文件CBA压缩失败): execute_compress失败时创建FailedTask(Compression类型),记录input_path、output_path、sesno_range,metadata包含file_name、db_num、is_new_file

- `src/data_interface/increment_manager.rs:942-958` (增量CBA压缩失败): 增量更新后的CBA压缩失败处理,创建Compression类型任务

### 重试机制

- `src/data_interface/increment_manager.rs:1257-1320` (retry_failed_task): 根据任务类型执行重试 - DatabaseQuery:调用query_latest_sesno_by_dbnum、Compression:解析sesno_range并调用execute_compress、IncrementUpdate和MqttPublish:TODO未完全实现

- `src/data_interface/increment_manager.rs:1325-1419` (start_retry_worker): 后台重试线程,每60秒扫描队列,获取待重试任务,执行重试,成功则remove,失败则schedule_next_retry并update,检查exhausted任务并告警

- `src/data_interface/tidb_manager.rs:62` (failed_queue字段): AiosDBManager新增Arc<FailedTaskQueue>字段

- `src/data_interface/db_model.rs:578-581` (队列初始化): 系统启动时创建FailedTaskQueue实例,路径为assets/failed_tasks.json

- `src/lib.rs:260` (worker启动): 主初始化流程中启动retry_worker后台线程

### Web监控接口

- `src/web_server/incremental_update_handlers.rs:14-29` (UpdateDetectionStatus): 增量更新检测状态枚举 - Idle、Scanning、ChangesDetected、Syncing、Completed、Error

- `src/web_server/incremental_update_handlers.rs:32-52` (IncrementalUpdateInfo): 增量更新信息结构 - site_id、site_name、last_sync_time、detection_status、pending_items、synced_items、changed_files、increment_size、estimated_sync_time

- `src/web_server/incremental_update_handlers.rs:54-67` (ChangedFile): 变更文件信息 - path、change_type(Added/Modified/Deleted)、size、modified_time、db_num

- `src/web_server/incremental_update_handlers.rs:78-100` (get_increment_sync_history_paged): 从SurrealDB的e3d_sync表查询历史记录,支持分页(limit/offset),SQL: SELECT file_names, file_hashes, timestamp, location, file_server_host, session_range, total_added, total_modified, total_deleted, is_full_sync, db_num FROM e3d_sync ORDER BY timestamp DESC

### 同步控制中心

- `src/web_server/sync_control_center.rs:28-32` (SYNC_EVENT_TX): 同步事件广播通道,broadcast::channel容量1000

- `src/web_server/sync_control_center.rs:34-36` (SYNC_CONTROL_CENTER): 全局同步控制中心实例,Arc<RwLock<SyncControlCenter>>

- `src/web_server/sync_control_center.rs:69-102` (SyncControlState): 同步控制状态 - is_running、is_paused、current_env、mqtt_connected、watcher_active、total_synced、total_failed、pending_count、queue_size、sync_rate_mbps、avg_sync_time_ms、uptime_seconds等统计指标

- `src/web_server/sync_control_center.rs:128-149` (SyncTask): 同步任务信息 - id、file_path、file_size、file_name、file_hash、env_id、source_env、target_site、direction、status、priority、retry_count、created_at/started_at/completed_at、error_message

### 实时进度广播

- `src/shared/progress_hub.rs:23-27` (ProgressHub): 统一进度管理中心,使用channels(DashMap<String, broadcast::Sender<ProgressMessage>>)管理每个任务的广播通道,task_states(DashMap<String, ProgressMessage>)存储任务状态

- `src/shared/progress_hub.rs:35-59` (ProgressMessage): 进度消息结构 - task_id、status(TaskStatus)、progress(0.0-100.0)、message、details、error、started_at/updated_at

- `src/shared/progress_hub.rs:64-75` (TaskStatus): 任务状态枚举 - Pending、Running、Completed、Failed、Cancelled

- `src/shared/progress_hub.rs:110-138` (register): 注册任务并返回订阅通道,如果任务不存在则创建新通道并初始化状态为Pending

- `src/shared/progress_hub.rs:144-157` (subscribe): 订阅任务进度更新,返回broadcast::Receiver

- `src/shared/progress_hub.rs:163-186` (publish): 发布进度消息到指定任务的通道,同时更新task_states,返回订阅者数量

- `src/shared/progress_hub.rs:189-193` (get_task_state): 获取任务当前状态快照

- `src/shared/progress_hub.rs:220-224` (all_task_states): 获取所有任务状态

- `src/shared/progress_hub.rs:252-328` (ProgressMessageBuilder): 进度消息构建器,提供链式调用API构建ProgressMessage

### 增量更新事件广播

- `src/data_interface/increment_manager.rs:1195-1203` (Running状态广播): 增量更新开始时通过ProgressHub发布Running状态消息

- `src/data_interface/increment_manager.rs:1321-1350` (Completed状态广播): 增量更新完成后发布Completed消息,包含details(added/modified/deleted统计、sesno范围、db_num等详细信息)

### 远程同步Handler

- `src/web_server/remote_sync_handlers.rs:19-36` (RemoteSyncEnv): 远程同步环境配置 - id、name、mqtt_host、mqtt_port、file_server_host、location、location_dbs、reconnect_initial_ms、reconnect_max_ms

- `src/web_server/remote_sync_handlers.rs:38-51` (RemoteSyncSite): 远程站点配置 - id、env_id、name、location、http_host、dbnums、notes

- `src/web_server/remote_sync_handlers.rs:53-72` (RemoteSyncLogRecord): 同步日志记录 - id、task_id、env_id、source_env、target_site、direction、file_path、status、error_message、started_at/completed_at

### 持久化存储

- `assets/failed_tasks.json`: 失败任务队列持久化文件,JSON数组格式,每个元素为一个FailedTask,包含完整的任务信息、时间戳、元数据

- `deployment_sites.sqlite`: 远程同步配置数据库,包含remote_sync_envs、remote_sync_sites、remote_sync_logs表

- `SurrealDB e3d_sync表`: 增量同步历史记录,字段包括file_names(文件名列表)、file_hashes(哈希列表)、timestamp、location(源站点)、file_server_host、session_range、total_added/modified/deleted、is_full_sync、db_num

- `SurrealDB dbnum_info_table`: PDMS元素数据存储,字段包括refno、dbnum、attr(属性)、children(子元素)、sesno(会话号)

### 压缩与哈希

- `pdms_io/sync/compress.rs` (execute_compress): 执行CBA压缩,输入PDMS数据库文件,输出.cba压缩包,返回SHA256文件哈希

- `docs/INCREMENT_DETECTION_FLOWCHART.md:643-651` (execute_compress流程): 创建CompressOptions、执行压缩、计算SHA256哈希、返回hash字符串

### MQTT推送

- `src/mqtt_service/mod.rs:10-29` (SyncE3dFileMsg): MQTT消息结构 - file_names(文件名列表)、file_hashes(哈希列表)、file_server_host、location(源站点)、timestamp

- `docs/INCREMENT_DETECTION_FLOWCHART.md:718-737` (MQTT推送逻辑): 构建SyncE3dFileMsg、INSERT INTO e3d_sync记录、mqtt_client.publish("Sync/E3d", QoS::ExactlyOnce, payload)

### 去重机制

- `docs/INCREMENT_DETECTION_FLOWCHART.md:684-701` (去重查询): SELECT * FROM e3d_sync WHERE location != '当前地区' AND '文件名' IN file_names AND '哈希' IN file_hashes,结果为空则推送,否则跳过避免重复

### 任务调度

- `src/web_server/sync_control_center.rs` (enqueue_generated_sync_tasks): 将生成的同步产物转换为同步任务,查询remote_sync_envs和remote_sync_sites获取目标站点,构建NewSyncTaskParams,调用SYNC_CONTROL_CENTER.add_task添加到队列

- `docs/INCREMENT_UPDATE_FLOW_ANALYSIS.md:487-551` (enqueue流程): 读取env_id、查询SQLite获取env_name和site_entries、遍历artifacts和targets、构建NewSyncTaskParams(file_path、file_size、priority=5、direction="UPLOAD")、添加到SyncControlCenter

---

## Report (The Answers)

### result

#### 1. 增量更新的数据流和状态管理

**增量更新任务创建和执行位置**:
- **创建**: `increment_manager.rs:async_watch()`监听文件系统事件,检测到文件变化后在Line 496-516对比会话号,确认有增量后构建increment_ranges_map
- **执行**: `increment_manager.rs:execute_incr_update()`负责核心处理,调用PdmsIO收集增量元素并更新到SurrealDB

**任务状态种类**:
- **UpdateDetectionStatus** (增量检测): Idle、Scanning、ChangesDetected、Syncing、Completed、Error(String)
- **TaskStatus** (通用任务): Pending、Running、Completed、Failed、Cancelled
- **SyncTaskStatus** (同步任务): 待查询具体定义,但基于SyncTask结构推断包含进行中、完成、失败等状态
- **FailedTask状态**: 由retry_count与max_retries对比得出 - 待重试(should_retry)、等待中(未到重试时间)、已耗尽(is_exhausted)

**状态存储和更新位置**:
- **失败任务状态**: FailedTaskQueue内存队列(Arc<RwLock<Vec<FailedTask>>>)并持久化到assets/failed_tasks.json
- **增量检测状态**: 通过ProgressHub.task_states(DashMap)存储,每个任务对应一个ProgressMessage
- **同步任务状态**: SYNC_CONTROL_CENTER内部状态(待查具体实现),通过SYNC_EVENT_TX广播状态变化
- **历史记录**: SurrealDB e3d_sync表存储增量同步历史,deployment_sites.sqlite存储远程同步日志

**统一任务管理中心**:
- **ProgressHub**: 统一进度管理中心,所有任务进度通过register注册、publish发布、subscribe订阅,提供单一数据源
- **SYNC_CONTROL_CENTER**: 同步任务管理中心,Arc<RwLock<SyncControlCenter>>全局实例,管理同步任务队列和状态
- **FailedTaskQueue**: 专门管理失败任务的队列系统,提供重试调度和持久化

#### 2. FailedTaskQueue的详细实现

**完整API**:
- **创建**: `new(persist_path)` - 创建队列并自动加载持久化任务
- **添加**: `push(task)` - 添加单个任务,`push_batch(tasks)` - 批量添加
- **查询**: `get_pending_tasks()` - 获取待重试任务,`get_all_tasks()` - 获取所有任务,`get_exhausted_tasks()` - 获取已耗尽任务
- **统计**: `get_stats()` - 返回TaskQueueStats(total/pending/waiting/exhausted)
- **更新**: `update(task)` - 更新任务状态,`remove(task_id)` - 移除成功任务
- **清理**: `cleanup_exhausted()` - 手动清理已耗尽任务,返回清理数量
- **持久化**: `persist()` - 异步持久化到JSON,`load_from_disk(path)` - 从JSON加载

**任务数据结构**:
- **FailedTask**: id(UUID字符串)、task_type(FailedTaskType枚举)、error(错误信息字符串)、error_trace(可选堆栈)、retry_count(u32)、max_retries(u32,默认5)、first_failed_at/last_retry_at/next_retry_at(SystemTime)、priority(u8,1-10)、metadata(Option<serde_json::Value>)
- **FailedTaskType**: DatabaseQuery{dbnum, operation}、Compression{input_path, output_path, sesno_range}、IncrementUpdate{path, sesno_range, dbnum}、MqttPublish{topic, payload_summary}

**获取不同状态任务的方法**:
- **待重试**: `get_pending_tasks()` - 过滤should_retry()为true的任务(retry_count < max_retries 且 当前时间 >= next_retry_at)
- **等待中**: 通过`get_stats()`获取waiting数量,计算为total - pending - exhausted
- **已完成**: 任务成功后通过`remove(task_id)`移除,不保留在队列中
- **已耗尽**: `get_exhausted_tasks()` - 过滤is_exhausted()为true的任务(retry_count >= max_retries)

**持久化机制**:
- **文件路径**: assets/failed_tasks.json
- **格式**: JSON数组,每个元素为序列化的FailedTask
- **原子写入**: 先写临时文件(.tmp扩展名),再重命名到目标文件,避免写入过程中断导致数据损坏
- **自动加载**: 队列创建时检测文件存在则自动加载,支持断电恢复
- **触发时机**: push、push_batch、update、remove、cleanup_exhausted操作后自动调用persist()

#### 3. 增量更新的元数据

**每个增量更新包含的信息**:
- **基本信息**: db_num(数据库编号)、old_sesno(原会话号)、new_sesno(新会话号)、session_range(会话号范围字符串,如"12341..=12350")
- **refno列表**: 通过collect_increment_eles收集,返回Vec<IncrementInfo>,每个IncrementInfo包含refno、db_no、attr、children、operation
- **文件信息**: path(数据库文件路径)、file_name(CBA文件名)、file_size(压缩包大小)、file_hash(SHA256校验和)
- **统计数据**: total_added(新增元素数)、total_modified(修改元素数)、total_deleted(删除元素数)、record_count(总元素数)
- **时间戳**: generated_at(生成时间)、timestamp(推送时间)
- **同步标记**: is_full_sync(布尔值,标识是否全量同步)

**增删改分类数据记录**:
- **收集阶段**: PdmsIO::collect_increment_eles返回的IncrementInfo包含operation字段(EleOperation::Add/Modified/Deleted)
- **统计阶段**: execute_incr_update处理后统计added/modified/deleted数量
- **存储位置**:
  - GeneratedSyncArtifact结构保存total_added/total_modified/total_deleted
  - SurrealDB e3d_sync表存储这些统计字段
  - ProgressMessage的details字段(JSON)包含增删改统计

**历史记录表**:
- **SurrealDB e3d_sync表**: 主要历史记录,字段包括file_names、file_hashes、timestamp、location、file_server_host、session_range、total_added、total_modified、total_deleted、is_full_sync、db_num,通过get_increment_sync_history_paged查询并支持分页
- **deployment_sites.sqlite remote_sync_logs表**: 远程同步日志记录,字段包括task_id、env_id、source_env、target_site、direction、file_path、status、error_message、started_at、completed_at
- **SurrealDB db_file_info表**: 文件会话号书签,记录每个文件处理到的sesno,用于增量检测

#### 4. 现有的Web监控界面

**已有的增量更新相关Web页面**:
- **增量更新历史页面**: 通过incremental_update_handlers.rs的get_increment_sync_history_paged handler提供,查询e3d_sync表并返回JSON,支持分页
- **增量更新信息接口**: IncrementalUpdateInfo结构定义了返回格式,包含site_id、detection_status、pending_items、changed_files等字段
- **变更文件列表**: ChangedFile结构提供path、change_type、size、modified_time、db_num信息

**remote_sync_handlers.rs功能**:
- **环境和站点管理**: RemoteSyncEnv和RemoteSyncSite结构定义远程同步配置
- **元数据加载**: MetadataQuery支持refresh参数,从远程站点或缓存加载元数据
- **站点信息查询**: SiteInfo结构聚合env和site信息
- **日志记录**: RemoteSyncLogRecord提供同步日志查询

**incremental_update_handlers.rs功能**:
- **历史记录查询**: 从e3d_sync表查询增量同步历史,返回file_names、file_hashes、timestamp、location、统计数据等
- **状态定义**: UpdateDetectionStatus枚举定义增量检测各阶段状态
- **变更跟踪**: ChangedFile和ChangeType提供文件变更详情

**现有实时监控机制**:
- **SSE (Server-Sent Events)**: 通过SYNC_EVENT_TX广播通道发送SyncEvent,客户端订阅/events端点接收实时更新
- **ProgressHub**: 统一进度管理,客户端订阅特定任务ID获取实时进度,支持WebSocket或轮询
- **广播通道**: broadcast::channel容量1000,支持多个订阅者同时接收事件
- **状态快照**: get_task_state和all_task_states提供当前状态查询,无需等待事件推送

**可复用的组件和模式**:
- **ProgressHub模式**: 注册任务→订阅通道→发布消息→广播到所有订阅者,适用于任何需要实时进度反馈的场景
- **FailedTaskQueue模式**: 失败任务入队→后台重试worker→指数退避→成功移除/失败更新,可复用到其他需要错误恢复的模块
- **原子持久化模式**: 临时文件→重命名,确保数据一致性
- **分页查询模式**: get_increment_sync_history_paged使用LIMIT/START实现分页,适用于大数据集查询
- **统计聚合模式**: TaskQueueStats提供total/pending/waiting/exhausted分类统计,便于监控和告警

---

### conclusions

1. **增量更新系统采用分层架构**: 文件监听层(PdmsWatcher) → 增量检测层(会话号对比) → 数据处理层(collect_increment_eles) → 存储层(SurrealDB) → 分发层(MQTT/SYNC_CONTROL_CENTER)

2. **存在完善的错误恢复机制**: FailedTaskQueue提供4种失败类型、指数退避重试(1/2/4/8/16分钟)、JSON持久化支持断电恢复、最大重试5次后告警

3. **状态管理高度分散**: 至少存在3个独立的状态管理系统 - ProgressHub(通用任务进度)、SYNC_CONTROL_CENTER(同步任务)、FailedTaskQueue(失败任务),缺乏统一接口

4. **实时监控已初步实现**: 通过ProgressHub+broadcast channel提供SSE推送,increment_manager已集成进度广播(Line 1195、1321),但Web UI可能尚未完全对接

5. **元数据记录非常详细**: 每次增量更新记录包含sesno范围、增删改统计、文件哈希、时间戳、站点信息等,存储在e3d_sync表和remote_sync_logs表

6. **重试逻辑尚未完全实现**: DatabaseQuery和Compression类型已实现重试(Line 1257-1289),但IncrementUpdate和MqttPublish仅有框架(Line 1292-1318标注TODO)

7. **去重机制基于文件哈希**: 通过查询e3d_sync表检查file_names和file_hashes组合,避免重复推送相同内容

8. **支持地区隔离**: location_dbs配置过滤数据库编号,实现多地区分工处理,避免跨地区干扰

9. **任务优先级系统存在但未充分利用**: FailedTask和SyncTask都有priority字段(1-10),但调度逻辑未见优先级排序实现

10. **持久化采用多种存储**: JSON文件(failed_tasks.json,人类可读)、SQLite(deployment_sites.sqlite,关系查询)、SurrealDB(e3d_sync,分布式同步),各有侧重

---

### relations

**数据流关系**:
- PdmsWatcher.scan_db_headers() → increment_manager.async_watch() → execute_incr_update() → PdmsIO.collect_increment_eles() → PdmsIO.update_elements_to_database() → SurrealDB

**状态更新关系**:
- increment_manager执行增量更新 → ProgressHub.publish(Running) → 广播到订阅者
- 更新完成 → ProgressHub.publish(Completed) → 附带details统计信息
- 操作失败 → FailedTaskQueue.push() → persist()到JSON → retry_worker扫描 → retry_failed_task()

**同步任务关系**:
- execute_compress()生成CBA文件 → GeneratedSyncArtifact → enqueue_generated_sync_tasks() → 查询deployment_sites.sqlite → SYNC_CONTROL_CENTER.add_task() → SYNC_EVENT_TX.send()

**去重查询关系**:
- execute_compress()返回file_hash → 查询e3d_sync表(WHERE file_names IN ? AND file_hashes IN ?) → 结果为空时INSERT INTO e3d_sync → mqtt_client.publish()

**Web查询关系**:
- incremental_update_handlers.get_increment_sync_history_paged() → SurrealDB.query("SELECT ... FROM e3d_sync") → 返回JSON数组 → 前端渲染历史列表

**重试流程关系**:
- retry_worker每60秒 → FailedTaskQueue.get_pending_tasks() → retry_failed_task(task) → 成功:queue.remove() / 失败:task.schedule_next_retry() + queue.update()

**监控订阅关系**:
- 前端调用ProgressHub.subscribe(task_id) → 获取broadcast::Receiver → 循环接收ProgressMessage → 更新UI
- increment_manager.publish(ProgressMessage) → ProgressHub内部通道广播 → 所有订阅者接收

**配置加载关系**:
- increment_manager.init_watcher() → 读取DbOption.toml → 提取watch_dirs、location_dbs、manual_db_nums → 过滤数据库文件 → 初始化headers

**持久化关系**:
- FailedTaskQueue.push() → tasks.write().push() → persist() → tokio::fs::write(temp_file) → tokio::fs::rename(temp_file, persist_path)
- 系统重启 → FailedTaskQueue::new() → load_from_disk() → serde_json::from_str() → 恢复内存队列

**元数据聚合关系**:
- IncrementInfo.operation(Add/Modified/Deleted) → execute_incr_update统计 → GeneratedSyncArtifact.total_added/modified/deleted → e3d_sync表 → ProgressMessage.details
