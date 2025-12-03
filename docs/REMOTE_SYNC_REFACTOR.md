## TODO: Remote Sync Reliability Refactor

### 1. Generated Sync Task Persistence
- **Goal**: Prevent loss of `.cba` bundles when REMOTE_RUNTIME 或 SQLite 不可用。
- **Changes**:
  - 在 `src/data_interface/increment_manager.rs` 新增 `PENDING_SYNC_QUEUE_PATH = "assets/pending_sync_tasks.json"`。
  - 实现 `load_pending_sync_artifacts` / `persist_pending_sync_artifacts`：负责从 JSON 读取/写入 `Vec<GeneratedSyncArtifact>`。
  - `GeneratedSyncArtifact` 结构扩展：`db_num`, `db_path`, `old_sesno`, `new_sesno`, `session_range`, `generated_at`。
  - `enqueue_generated_sync_tasks` 改为先 append 到 pending 队列，再调用 `try_enqueue_sync_tasks`；失败时记录错误并保留 JSON，成功后清空。

### 2. MQTT Publish Retry
- **Goal**: 提高 `Sync/E3d` 推送在网络波动/断线时的可靠性。
- **Changes**:
  - 实现 `publish_sync_payload_with_retry(client: AsyncClient, payload: SyncE3dFileMsg)`；最大 3 次重试，退避 0.5s/1s/1.5s。
  - `async_watch` 里所有 `mqtt_client.publish` 调用改为使用该 helper，失败只写日志，不 panic。

### 3. File Watch Debounce
- **Goal**: 减少频繁保存导致的重复扫描和空增量。
- **Changes**:
  - 在 `async_watch` 中维护 `HashMap<PathBuf, Instant>`，常量 `FILE_EVENT_DEBOUNCE_MS = 500`。
  - 同一路径 500ms 内收到的事件直接跳过，确保文件稳定后再解析 header。

### 4. Metadata for New/Incremental Files
- **Goal**: 确保新增 DB 文件和增量任务包含完整上下文。
- **Changes**:
  - 新增文件：在 `params` 中插入 `1..=current_sesno` 的范围；压缩 `.cba` 时写入 `GeneratedSyncArtifact` 的 `session_range="1-{sesno}"`, `generated_at=SystemTime::now()`。
  - 现有文件增量：记录 `old_sesno`, `new_sesno`, `session_range="{prev+1}-{new}"`, `record_count=delta`。
  - 所有 `GeneratedSyncArtifact` 均携带 `db_num`, `db_path`，便于 UI/任务中心展示。

### 5. CHANGELOG & Ops Doc
- 更新 `CHANGELOG.md`：描述本次 refactor 的核心要点（任务持久化、去抖、MQTT 重试、元数据补充）。
- 保持本文件（`docs/REMOTE_SYNC_REFACTOR.md`）与 `CLAUDE.md` 同步，作为后续实现/评审参考。
