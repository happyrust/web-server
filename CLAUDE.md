# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is **aios-database**, a Rust-based 3D plant engineering data management system that processes AVEVA PDMS/E3D databases and provides web-based visualization, incremental updates, and remote synchronization capabilities. The system bridges legacy PDMS data with modern graph databases (SurrealDB), spatial indexing, and real-time collaboration features.

### Build Commands

```bash
# Check compilation (fastest)
cargo check

# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Build specific binary
cargo build --bin web_server --features web_server

# Linux cross-compilation (CentOS 7 compatibility)
cargo zigbuild --release --target x86_64-unknown-linux-gnu.2.17
```

### Test Commands

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test test_incr_update

# Run integration tests
cargo test --test unit --features web_server

# Run remote sync smoke test
cargo test --bin remote_sync_smoke_test --features web_server -- --nocapture

# Run with debug logging
RUST_LOG=debug cargo test --features web_server -- --nocapture
```

### Running the Web Server

```bash
# Start web UI (reads DbOption.toml configuration)
cargo run --bin web_server --features web_server

# Start with custom configuration
cargo run --bin web_server --features web_server -- --config DbOption_custom.toml
```

### Development Scripts

```bash
# Start test database
./scripts/start_test_db.sh

# Run model generation tests
./scripts/run_model_test.sh

# Test XKT correctness
./scripts/test_xkt_correctness.sh

# Complete 1112 database test
./scripts/test_1112_complete.sh
```

## High-Level Architecture

### Core Module Structure

The codebase is organized into domain-driven modules under `src/`:

**Data Layer**:
- `data_interface/` - Database abstraction layer
  - `tidb_manager.rs` - MySQL/TiDB operations (AiosDBManager)
  - `db_manager.rs` - General database management
  - `increment_manager.rs` - Incremental update detection and processing
  - `mesh_manager.rs` - 3D mesh data handling
- `versioned_db/` - Version control for PDMS databases

**Model Generation** (`fast_model/`):
- Converts PDMS element data to 3D geometry and metadata
- Query providers: Unified interface (`query_provider.rs`) abstracts SurrealDB queries
- Model generators: `cata_model.rs` (catalogues), `prim_model.rs` (primitives), `loop_model.rs` (loops)
- `mesh_generate.rs` - Triangle mesh generation from PDMS primitives
- `room_model_v2.rs` - Spatial room relationship calculation

**Web Server** (`web_server/`):
- Axum-based HTTP API and SSE (Server-Sent Events)
- Handlers: `incremental_update_handlers.rs`, `remote_sync_handlers.rs`, `sync_control_handlers.rs`, `dashboard_handlers.rs`
- `sync_control_center.rs` - Central coordinator for remote site synchronization
- `dashboard_handlers.rs` - Real-time monitoring dashboard with 7 API endpoints (summary, active-tasks, failed-tasks, timeline stats, retry, cleanup)
- `dashboard_template.rs` - Alpine.js-based dashboard UI with Chart.js visualizations
- WebSocket support in `ws/` module for real-time task status updates
- Static files served from `src/web_server/static/` (including `dashboard.js`, `dashboard.css`)

**Spatial Indexing**:
- `spatial_index.rs` - R-tree and AABB-based spatial queries
- `sqlite_index.rs` - SQLite-backed spatial persistence

**External System Integration**:
- `mqtt_service/` - MQTT pub/sub for distributed events
- `grpc_service/` - gRPC APIs (optional feature)

### Key Design Patterns

**Database Abstraction**: `AiosDBManager` provides unified CRUD operations across MySQL, SQLite, and SurrealDB. Query results use typed extractors to avoid direct SQL/Surreal value manipulation.

**Incremental Updates**: The system detects PDMS "session number" changes in `.cdf` files, computes delta ranges, and generates `.cba` compressed archives containing only modified elements. See `increment_manager.rs:detect_increment()` and `docs/INCREMENT_DETECTION_FLOWCHART.md`.

**Remote Sync Workflow**:
1. `notify` crate watches PDMS file changes
2. `SyncControlCenter` queues sync tasks with priority/retry logic
3. Tasks execute file transfer (local copy or HTTP upload)
4. MQTT publishes completion events to remote sites
5. Web UI receives SSE updates for real-time monitoring

**Model Generation Pipeline**:
1. Parse PDMS binary databases via `parse_pdms_db` crate
2. Load element attributes into SurrealDB (`pe` table)
3. Generate geometry per noun type (PANE, BRAN, etc.) in parallel
4. Apply CSG boolean operations if `apply_boolean_operation = true`
5. Build spatial AABB tree and room relationships
6. Export meshes to `.glb`/`.gltf` or XKT format

### Configuration System

Primary config: `DbOption.toml` in repository root. Key sections:

- **Project paths**: `project_path`, `included_projects`, `project_code`
- **Model generation**: `gen_model`, `gen_mesh`, `gen_spatial_tree`, `mesh_tol_ratio`
- **Full Noun mode**: `full_noun_mode = true` enables parallel noun-type processing (ignores `manual_db_nums`)
- **Database backends**: `sync_tidb`, `sync_graph_db`, `sync_live` (SurrealDB)
- **Remote sync**: `remote_file_server_hosts`, `file_server_host`
- **Debug options**: `debug_limit_per_noun`, `debug_refno_types`

Do not commit sensitive paths. Copy `DbOption.toml` to a local variant for development.

## Code Patterns and Conventions

### Rust Features

The project requires **nightly Rust** with these unstable features:
```rust
#![feature(let_chains)]
#![feature(async_closure)]
#![feature(exact_size_is_empty)]
#![feature(slice_take)]
#![feature(const_async_blocks)]
#![feature(type_alias_impl_trait)]
```

All new code must compile with these features enabled.

### SurrealDB Query Patterns

**IMPORTANT**: Do NOT use raw `.query().await?.take()` patterns. Always use the project's extension methods:

```rust
use surrealdb::types as surrealdb_types;
use crate::fast_model::query_provider::SurrealQueryExt;

// ✅ CORRECT: Use query_take for single-result queries
let refnos: Vec<RefNo> = db.query_take("SELECT REFNO FROM pe WHERE noun = 'SITE'", 0).await?;

// ✅ CORRECT: Use query_response for multi-result queries
let response = db.query_response("SELECT * FROM pe LIMIT 10; SELECT count() FROM pe;").await?;
let data: Vec<PeData> = response.take(0)?;
let count: i64 = response.take(1)?;

// ❌ WRONG: Direct unwrapping loses error context
let data = db.query(sql).await?.take(0).unwrap();
```

These methods provide `#[track_caller]` error location tracking and proper anyhow integration.

### AVEVA PDMS Naming Conventions

PDMS uses domain-specific abbreviations. Refer to `docs/attlib_naming_conventions.md` for official terminology. Common examples:
- **Noun types**: `SITE`, `ZONE`, `EQUI`, `BRAN`, `PANE`, `SCTN`
- **Attributes**: `REFNO` (reference number), `SESNO` (session number), `PAXI` (axis), `BORE` (diameter)
- **Files**: `.cdf` (configuration), `.rvm` (review model), `.cba` (compressed binary archive)

Use exact PDMS terminology in variable names when mapping database fields.

### Error Handling

Use `anyhow::Result` for fallible operations. Propagate context with `.context()`:

```rust
use anyhow::{Context, Result};

fn load_database(path: &Path) -> Result<Database> {
    let data = fs::read(path)
        .context(format!("Failed to read database file: {}", path.display()))?;
    parse_db(&data).context("Database parsing failed")
}
```

### Testing Conventions

- Unit tests live in `src/test/` or inline `mod tests`
- Integration tests in `tests/` directory
- Test data fixtures in `test_data/`
- Use descriptive test names: `test_increment_detection_with_session_gap`
- Run tests with `--nocapture` for debugging: `cargo test -- --nocapture`

### Feature Flags

The project uses Cargo features extensively. Key features:

- `web_server` - Enables Axum HTTP server and web UI
- `gen_model` - Model generation pipeline
- `manifold` - CSG boolean operations (via `aios_core/manifold`)
- `sqlite-index` - Spatial indexing via SQLite
- `grpc` - gRPC server support
- `mqtt` - MQTT event publishing

When adding new code, consider which features should gate it. Use `#[cfg(feature = "...")]` attributes.

## Common Development Tasks

### Adding a New Handler to Web Server

1. Define handler function in `src/web_server/` (e.g., `my_feature_handlers.rs`)
2. Add template in `src/web_server/my_feature_template.rs` if needed
3. Register route in `src/web_server/mod.rs` `create_app()` function
4. Update `AppState` if new shared state is required
5. Add frontend assets to `src/web_server/static/`

### Implementing Incremental Update Detection

Reference `src/data_interface/increment_manager.rs`. Key steps:

1. Read file session number from `.cdf` metadata
2. Query current session from SurrealDB: `SELECT SESNO FROM pe ORDER BY SESNO DESC LIMIT 1`
3. If `file_sesno > db_sesno`, range is `(db_sesno+1)..=file_sesno`
4. Generate CBA: `aios_core::cba_utils::generate_cba(refnos, output_path)`
5. Log event to `deployment_sites.sqlite`

See `docs/INCREMENT_DETECTION_FLOWCHART.md` for full algorithm.

### Running a Single Test

```bash
# Run specific test by name
cargo test test_task_priority_ordering --features web_server -- --exact

# Run all tests in a module
cargo test test_incr_update --features web_server

# Keep test database for inspection
# Comment out cleanup code, then check test_data/ directory
```

### Debugging Model Generation Issues

1. Enable debug logging: `RUST_LOG=debug cargo run --bin web_server --features web_server`
2. Check generated meshes in `meshes_path` directory (configured in `DbOption.toml`)
3. Use `debug_model_refnos` in config to process only specific elements
4. Reduce `debug_limit_per_noun` for faster iteration
5. Inspect SurrealDB data: `surreal sql --ns <project_code> --db <project_name>`

### Using the Real-Time Monitoring Dashboard

The dashboard provides comprehensive monitoring of incremental update tasks:

**Access**: Navigate to `http://localhost:8080/dashboard` after starting the web server

**Key Features**:
- Real-time task monitoring with WebSocket updates (<1s latency)
- Failed task queue management with retry and cleanup operations
- Historical trend charts (task counts, success rate, average duration)
- Time window selection (1h/24h/7d/30d)

**API Endpoints** (for integration):
```
GET  /api/dashboard/summary          - Overview statistics
GET  /api/dashboard/active-tasks     - Currently running tasks
GET  /api/dashboard/failed-tasks     - Failed task queue (filterable)
GET  /api/dashboard/stats/timeline   - Historical aggregated data
POST /api/dashboard/retry-task/:id   - Manual retry
POST /api/dashboard/cleanup-exhausted - Bulk cleanup
```

**Tech Stack**: Alpine.js 3.x, Chart.js 4.4, WebSocket, SQLite time-bucketing queries

See `llmdoc/guides/dashboard-usage-guide.md` for detailed usage instructions and `llmdoc/architecture/dashboard-architecture.md` for technical architecture.

## Important Development Notes

### Windows Compilation Troubleshooting

**Problem**: `aws-lc-sys` build errors
**Solution**: Set `AWS_LC_SYS_NO_ASM=1` and ensure cmake/nasm are in PATH

**Problem**: `bindgen` cannot find libclang
**Solution**: Set `LIBCLANG_PATH=D:\LLVM\bin` (adjust to your LLVM install path)

**Problem**: PATH not updating
**Solution**: Restart terminal or use `setup_build_env.ps1` script

### Test Data Management

Test fixtures in `test_data/` may be large binary files. Use Git LFS if adding new PDMS databases. Current test databases:
- `increment_patch/` - Sample incremental update archives
- `virtual_hole/` - Spatial query test fixtures

### Feature Branch Workflow

Current branch: `only-csg`
Main branch: `only-csg`

When creating PRs, target the main branch (`only-csg`). Ensure all tests pass and update documentation if adding new features.

### Performance Considerations

- Model generation is CPU-intensive. Use `gen_model_batch_size` to control parallelism
- SurrealDB queries should use indexes. Check query plans with `EXPLAIN`
- Large mesh generation can OOM. Monitor memory usage with `mesh_tol_ratio` adjustments
- Remote sync uses concurrent workers. Tune `max_concurrent_syncs` in `deployment_sites.sqlite`

## Documentation References

Key documentation in `docs/`:

- `REMOTE_SYNC_DEVELOPMENT_GUIDE.md` - Remote synchronization architecture
- `INCREMENT_DETECTION_FLOWCHART.md` - Increment detection algorithm
- `TESTING_QUICK_START.md` - Test strategy and examples
- `GUI_DESIGN_SPEC.md` - Web UI design specifications
- `PROGRESS_HUB_IMPLEMENTATION.md` - Real-time progress broadcasting

For external systems (PDMS/E3D), consult official AVEVA documentation.



## SurrealDB Types & Query Patterns
项目使用 SurrealDB 作为主要数据库，类型系统与查询模式遵循以下规范：

**类型别名与导入**：
- `surrealdb_types` 仅为 `surrealdb::types` 的模块别名
- 使用时需要导入模块别名：`use surrealdb::types as surrealdb_types;`
- 具体类型应使用完整路径导入：`use surrealdb::types::SurrealValue;`
- 查询接口通过 `SurrealQueryExt` trait 扩展 `Surreal<Any>`

**查询方法规范**：
- **禁止**直接使用 `.query().await?.take()` 并 unwrap，必须使用项目提供的扩展方法
- **推荐**使用 `query_take::<T>(sql, index)` 执行查询并反序列化第 `index` 个结果
- 使用 `query_response(sql)` 获取完整的 `Response` 对象以便多结果处理
- 所有查询方法已集成 `#[track_caller]` 实现精确错误定位

**类型约束与转换**：
- 查询目标类型 `T` 必须满足 `T: SurrealValue` 和 `usize: SurrealQueryResult<T>`
- 反序列化失败会通过 `anyhow::Error` 传播，并附带 SQL 语句和调用位置信息
- 优先使用具体类型（如 `Vec<RefNo>`）而非手动解析 `SurrealValue` 枚举

**使用示例**：
```rust
use surrealdb::types as surrealdb_types;
use surrealdb::types::SurrealValue;
use crate::rs_surreal::query_ext::SurrealQueryExt;

// 单结果查询
let result: Vec<RefNo> = db.query_take("SELECT REFNO FROM pe WHERE noun = 'SITE'", 0).await?;

// 多结果查询
let response = db.query_response("SELECT * FROM pe LIMIT 10; SELECT count() FROM pe;").await?;
let data: Vec<PeData> = response.take(0)?;
let count: i64 = response.take(1)?;
```

## Testing Guidelines
测试框架依赖 Cargo 内建机制，建议在本地同时执行 `cargo test` 与 `cargo test --lib` 对比输出；针对数据库差异，可运行 `cargo run --example test_unified_query` 并比对 `surreal_perf.log`；独立模块可使用 `cargo test test_memory_database_init`、`cargo test test_gensec_spine -- --nocapture` 等现有命令作为模板；新增集成夹具放入 `test-files/`，输出日志放入 `test_output/`，文件命名遵循 `test_模块_场景.log` 以便归档。

## Commit & Pull Request Guidelines
历史记录采用类 Conventional Commit 规范，如 `feat: 重构 surreal 查询缓存`、`test: add simplified RefnoEnum tests`，建议继续使用 `feat|fix|refactor|test|docs` 前缀描述范围；提交信息需概括影响模块与动机，并在需要时引用 issue 或阶段性文档；创建 PR 前请附测试命令列表、关键日志或截图（可引用 `docs/`、`test_output/` 中的材料），确认夜间与 release 构建均通过；若改动影响 `examples/` 或外部脚本，请在说明中标注并更新相应使用文档。

## Configuration & Safety Notes
连接配置位于 `DbOption.toml` 与 `DbOption_*.toml`，请使用本地副本并避免提交真实凭据；示例依赖的资产与中间结果存放在 `resource/`、`data/`、`all_attr_info.*`，拉取前确认体积较大的二进制已同步；日志与性能对比文件建议留在忽略目录，避免污染仓库，同时注意清理 `target/` 以减少版本库噪音。
