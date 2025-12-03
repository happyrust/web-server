# Changelog

## 2025-12-01

### Changed
- **重构 PdmsWatcher 文件路径映射**
  - `pdms-io/src/watch.rs`: 重命名 `file_name_full_path_map` 为 `db_path_map`
  - 添加 `is_valid_e3d_db_file()` 过滤函数，只扫描无扩展名的有效 E3D 数据库文件
  - 新增 `get_db_path()` 和 `insert_db_path()` 方法，key 统一使用 UPPERCASE 保证存取一致性
  - `src/data_interface/db_model.rs`: 使用 `watcher.get_db_path()` 替代直接访问 map
  - `src/data_interface/increment_manager.rs`: 使用 `watcher.insert_db_path()` 替代直接 insert

- **改进拓扑可视化连接显示**
  - `frontend/src/components/views/TopologyVisualization.vue`: 
    - 从 `/api/topology` 加载拓扑配置，显示所有配置的主从连接
    - 用不同颜色区分连接状态：绿色(已订阅)、蓝色(在线未订阅)、灰色虚线(离线)
    - 合并 MQTT 节点状态和拓扑配置节点，确保所有配置的节点都显示

## 2025-11-18

### Added
- **持久化远程同步任务队列**
  - `src/data_interface/increment_manager.rs`: 新增 `assets/pending_sync_tasks.json` 队列，`enqueue_generated_sync_tasks` 在 REMOTE_RUNTIME/SQLite 不可用或写入失败时自动落盘并在下一次触发时重试，彻底避免网络抖动造成的增量包丢失。

- **增强同步任务元数据追踪**
  - `GeneratedSyncArtifact` 结构体扩展字段：
    - `db_num`: 数据库编号
    - `db_path`: 数据库文件路径
    - `old_sesno` / `new_sesno`: 会话号范围（旧值/新值）
    - `session_range`: 格式化的会话范围字符串（如 "1-100" 或 "50-60"）
    - `generated_at`: 任务生成时间戳
    - `is_full_sync`: 标记是否为全量同步（新文件）
  - 新增文件自动标记为全量同步，会话范围为 `1-{当前sesno}`
  - 增量文件标记为增量同步，会话范围为 `{old_sesno+1}-{new_sesno}`

- **MQTT 发送重试机制**
  - `src/data_interface/increment_manager.rs`: 新增 `publish_sync_payload_with_retry` 函数
  - 最多重试 3 次，指数退避延迟（500ms / 1000ms / 1500ms）
  - 所有 MQTT 发布调用统一使用重试机制，提高网络不稳定环境下的可靠性

### Changed
- **文件监听链路增加去抖与幂等保护**
  - `src/data_interface/increment_manager.rs`: `async_watch` 现对同一路径 500ms 内的重复事件直接忽略，并在 `Ok(false)` 时不再刷新 headers，确保频繁保存或空增量场景不会错过后续同步机会。
  - 去抖窗口常量 `FILE_EVENT_DEBOUNCE_MS = 500`

- **同步任务描述增强**
  - `try_enqueue_sync_tasks` 在生成任务注释时，优先使用 `session_range`，包含 DB 编号、会话范围、文件名、目标站点等详细信息
  - 改进日志输出，便于运维人员快速识别同步任务类型和范围

### Fixed
- **新文件全量导入修复**
  - 修复新增 DB 文件无法自动触发全量导入的问题
  - 新文件检测时自动将 `1..=current_sesno` 范围加入 `params`，确保 `execute_incr_update` 被调用
  - 生成的 artifact 正确标记 `is_full_sync: true` 和 `session_range: "1-{sesno}"`

- **Feature Gate 修复**
  - 将新文件 CBA 生成的条件编译从 `#[cfg(feature = "mqtt")]` 改为 `#[cfg(any(feature = "mqtt", feature = "web_server"))]`
  - 确保仅启用 `web_server` 特性时也能正常生成同步归档

- **远程同步推送更可靠**
  - MQTT 发布失败时记录详细错误日志，不再直接 panic
  - 新增的同步任务在队列写入失败时会提示具体原因，便于快速排查
  - 增量检测过程中，所有生成的 artifact 都包含完整元数据（包括 `db_num`, `session_range`, `is_full_sync` 等），避免信息丢失
  - 任务描述 notes 包含"全量同步"或"增量同步"标签，便于运维识别

## 2025-11-07

### Fixed
- **增量同步的返回语义和错误处理一致化**
  - `src/data_interface/increment_manager.rs`：`execute_incr_update` 现在根据是否真正写入增量返回 `Ok(true/false)`，并在 sesno 范围为空、集合为空的场景跳过数据库写入，避免调用方误判。
  - 同一函数在更新 `db_file_info` 时加入错误捕获，防止 SurrealDB 异常导致监听任务 panic。
- **缺失会话号记录时的全量补齐**
  - `init_watcher` 在 `dbnum_info_table` 没有记录或返回 0 时，不再直接 `continue`，会打印提示并自动从 `sesno=1` 起补齐；同时对 “nearest sesno” 逻辑做了边界保护，避免越界区间。
- **文件监听链路错误兜底**
  - `async_watch`：对 Pdms 头扫描结果转为 `Vec` 避免所有权问题；更新增量区间时重新拉取数据库最新 sesno 并做 `start_sesno` 校验；增量完成后若返回 `Ok(false)` 也会同步 headers，防止下一次重复触发。
  - 重新生成 CBA、查询 `e3d_sync`、记录 MQTT 推送等关键步骤全部改为 `match` 处理并输出中文错误日志，出现失败时跳过当前文件而不是整体崩溃。

### Changed
- **日志与提示更贴合运维**
  - 启动阶段会明确打印“数据库缺少 db_no 记录，准备从头导入”。
  - 新增文件及增量推送流程对每一步都补充了中文上下文，便于排查现场问题。

## 2025-10-27

### Fixed
- **修复 SurrealDB 查询错误 "Expected any, got record"**
  - 问题：在多个查询中使用 `in.id != none` 条件导致 SurrealDB 执行记录存在性检查，触发类型不匹配错误
  - 影响：导致模型生成过程中 panic，错误信息为 "更新模型数据失败: Internal error: Expected any, got record"
  - 修复位置：
    - `src/fast_model/manifold_bool.rs` (第 79-85, 286-290 行) - 移除 2 处 `in.id != none` 条件
    - `src/fast_model/occ_generate.rs` (第 705-713 行) - 移除 `in.id != none` 条件
    - `src/web_server/handlers.rs` (第 1730-1735 行) - 移除 `in.id != none` 条件
  - 原理：`inst_relate` 关系表的 `in` 字段总是指向有效的 `pe:{refno}`，不需要额外的存在性检查
  - 结果：程序现在可以正常运行，成功处理 GLB 模型导出

### Changed
- **优化 SurrealDB 查询性能**
  - 移除冗余的 `in.id != none` 检查条件，减少不必要的数据库操作
  - 简化查询逻辑，提升查询效率

## 2025-10-16

### Fixed
- **修复 `get_ancestor_attmaps` 中 NONE 值导致的反序列化失败问题**
  - 问题：`fn::ancestor({}).refno.*` 查询返回的祖先链中包含 NONE 值（当节点的属性记录不存在时），导致 `try_into::<NamedAttrMap>()` 反序列化失败
  - 影响：导致 `get_world_transform` 无法获取世界变换，几何体生成被跳过
  - 修复：在 `AttributeQueryService::get_ancestor_attmaps` 和 `query.rs::get_ancestor_attmaps` 中使用 `filter_map` + `try_into().ok()` 过滤掉无法转换的 NONE 值
  - 相关提交：rs-core@2dd7c11, gen-model@f41002f4

### Added
- **为 `gen_prim_geos` 添加详细调试日志**
  - 添加函数入口/出口日志，记录总数量、批次策略
  - 添加每批次的详细执行日志（开始、完成、耗时）
  - 添加详细的错误处理日志，区分不同跳过原因（世界变换失败、brep_shape 创建失败等）
  - 添加实体处理进度跟踪（处理数、跳过数、发送次数）
  - 使用 `e3d_dbg!` 宏统一调试日志输出

## 2025-02-14

- 将 `external/rs-corel` 中的连接、类型封装全部合并进 `external/rs-core`。
- 主项目直接使用 `aios_core` 公开的运行时 / WebUI 接口，移除独立 `rs-corel` 依赖。
- Web UI 及增量数据模块改为使用 `aios_core` 的 `RecordId`、`Datetime` 与连接工具。
- 构建脚本验证通过（`cargo check`）。
