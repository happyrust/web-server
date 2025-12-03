# Repository Guidelines

## 项目结构与模块组织
核心 Rust 代码位于 `src/`，其中 `lib.rs` 暴露通用库，`main.rs` 和 `src/bin/` 下的入口二进制负责 CLI 与 Web UI。`src/cata/`、`src/data_interface/` 等子模块以领域拆分功能，测试样例集中在 `src/test/` 与 `test_data/`。静态资源与示例数据分别放在 `assets/` 与 `data/`，自动化脚本位于 `scripts/` 与仓库根目录的 `run_*.sh` 文件。前端与可视化配套位于 `frontend/` 与 `js/`，`docs/` 保留面向外部的设计文档。

## 构建、测试与开发命令
`cargo build --all-features` 编译完整功能集，推荐在提交前执行以验证依赖特性互相兼容。`cargo run --bin web_server --features web_server` 启动内置 Web UI 调试界面，默认读取 `DbOption.toml` 中的配置。核心回归测试使用 `cargo test --all-targets`，并在需要重放模型导出流程时运行 `./run_model_gen_test.sh`。若需复现 XKT 生成链路，可执行 `./test_xkt_generation.sh` 或 `node test_complete_flow.js`。

## 代码风格与命名约定
Rust 代码统一采用四空格缩进并运行 `cargo fmt`，提交前请补充 `cargo clippy --all-targets --all-features` 以捕获潜在缺陷。模块命名保持蛇形命名（`snake_case`），公开类型与 trait 使用帕斯卡命名（`PascalCase`），配置文件沿用全小写短横线风格。脚本与 Node 模块遵循 ES 模块语法，文件名保持小写短横线（如 `generate_zone_demo.js`）。

## 测试准则
Rust 测试默认放在与实现同目录的 `mod tests` 或 `src/test/` 独立模块，命名以 `_test` 结尾突出意图。空间索引与数据库兼容性测试使用嵌入式 SQLite，因此本地运行前确保 `test_data/` 下示例数据库齐备。重现端到端模型生成时，先执行 `cargo build`，再调用 `./run_model_gen_test.sh` 以比对输出目录差异；必要时将生成的日志上传至 `logs/` 或 `output/` 目录便于复查。

## 提交与合并请求指南
Git 历史既包含 `feat:` 前缀的变更，也有简洁的命令式短句，建议统一采用动词开头的英文一句话摘要（如 `Add surreal cache warmup`）。若修复特定缺陷，请在正文引用关联 issue（`Fixes #123`）并概述验证方法。创建合并请求时附带功能说明、测试结果与必要的截屏或日志；涉及配置或数据格式更新时同时更新 `docs/` 与对应的 `DbOption*.toml` 注释。

## 配置与环境提示
默认配置文件位于仓库根目录的 `DbOption*.toml`，本地调试请复制 `DbOption.toml` 为私有版本并避免提交敏感路径。运行需要外部依赖（如 SurrealDB、MQTT 或 LiteFS）时，可使用 `start_surreal_with_check.sh` 与 `litefs-start.sh` 快速拉起必要服务；在 CI 环境请禁用 `--features web_server` 以缩短构建时间。


代码检索默认优先 Serena。
### 3.7 Serena 使用指南
Serena（本地代码分析+编辑优先）
**工具能力**：
- **符号操作**: find_symbol, find_referencing_symbols, get_symbols_overview, replace_symbol_body, insert_after_symbol, insert_before_symbol
- **文件操作**: read_file, create_text_file, list_dir, find_file
- **代码搜索**: search_for_pattern (支持正则+glob+上下文控制)
- **文本编辑**: replace_regex (正则替换，支持 allow_multiple_occurrences)
- **Shell 执行**: execute_shell_command (仅限非交互式命令)
- **项目管理**: activate_project, switch_modes, get_current_config
- **记忆系统**: write_memory, read_memory, list_memories, delete_memory
- **引导规划**: check_onboarding_performed, onboarding, think_about_* 系列
**触发场景**：代码检索、架构分析、跨文件引用、项目理解、代码编辑、重构、文档生成、项目知识管理
**调用策略**：
- **理解阶段**: get_symbols_overview → 快速了解文件结构与顶层符号
- **定位阶段**: find_symbol (支持 name_path 模式/substring_matching/include_kinds) → 精确定位符号
- **分析阶段**: find_referencing_symbols → 分析依赖关系与调用链
- **搜索阶段**: search_for_pattern (限定 paths_include_glob/restrict_search_to_code_files) → 复杂模式搜索
- **编辑阶段**:
  - 优先使用符号级操作 (replace_symbol_body/insert_*_symbol)
  - 复杂替换使用 replace_regex (明确 allow_multiple_occurrences)
  - 新增文件使用 create_text_file
- **项目管理**:
  - 首次使用检查 check_onboarding_performed
  - 多项目切换使用 activate_project
  - 关键知识写入 write_memory (便于跨会话复用)
- **思考节点**:
  - 搜索后调用 think_about_collected_information
  - 编辑前调用 think_about_task_adherence
  - 任务末尾调用 think_about_whether_you_are_done
- **范围控制**:
  - 始终限制 relative_path 到相关目录
  - 使用 paths_include_glob/paths_exclude_glob 精准过滤
  - 避免全项目无过滤扫描
- 工作顺序统一为：get_symbols_overview → find_symbol/find_referencing_symbols → 符号级编辑（replace_symbol_body/insert_before_symbol/insert_after_symbol）；避免整文件正则修改。
- 搜索统一用 search_for_pattern，启用 restrict_search_to_code_files=true，并记录所用过滤条件；禁止无范围的模糊查询。
- 思考节点强制：检索后 think_about_collected_information；编辑前 think_about_task_adherence；提交前 think_about_whether_you_are_done。
- 文件与目录仅用 list_dir/find_file 辅助定位；避免一次性读取大文件。
- 记忆默认不写入；仅当 docs/ 存在缺口或利害相关方明确要求时使用 write_memory，并在 coding-log.md 记录范围与时间。

# Repository Guidelines

## Project Structure & Module Organization
项目代码集中在 `src/`，其中 `aios_db_mgr` 与 `query_provider` 实现核心数据库桥接，`rs_surreal` 聚合 SurrealDB 适配，而 `geometry`、`material`、`version_control` 等模块支撑三维语义；命令入口存放于 `src/bin/` 与 `examples/`，演示如 `test_unified_query` 可直连双引擎；架构与同步方案记录在 `docs/`，性能数据、夹具与输出保存在 `benches/`、`test-files/`、`test_output/`，可复用现有 `.cypher`、`.json`、`.log` 文件；资源及二进制字典集中在 `resource/` 与 `data/`。

## Build, Test, and Development Commands
仓库使用 `rust-toolchain.toml` 固定 nightly，请先运行 `cargo check` 以验证依赖；常规构建采用 `cargo build --release` 生成高性能产物，调试阶段可执行 `cargo build`；单元与特性测试通过 `cargo test` 覆盖主要模块，针对查询管线可运行 `cargo test test_query_provider -- --nocapture`；集成路径借助 `cargo run --example test_unified_query` 验证 Surreal 流程；性能基准位于 `cargo bench --bench query_provider_bench`；生成文档使用 `cargo doc --open`。

## Coding Style & Naming Conventions
核心库启用了 `#![feature(let_chains, trivial_bounds, result_flattening)]`，提交前须确保新增代码在这些 feature 下可编译；执行 `cargo fmt --all` 保持官方 `rustfmt` 风格并遵循 4 空格缩进；模块与文件命名使用 `snake_case`（例如 `data_center.rs`），类型使用 `CamelCase`（如 `PdmsDatabaseInfo`），常量以 `SCREAMING_SNAKE_CASE` 命名；特性相关逻辑需通过 `#[cfg(feature = "live")]` 等条件编译明确包裹。

**外部系统命名约定**：AVEVA PDMS/E3D 系统的内部缩写和数据结构命名请参考 `docs/attlib_naming_conventions.md`。

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
