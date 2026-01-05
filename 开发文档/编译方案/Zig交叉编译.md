# Zig 交叉编译方案

本文档记录了如何使用 `zig` 作为链接器，在本地（macOS/Linux）交叉编译适用于 Linux x86_64 环境的 `web_server` 二进制程序。此方案解决了不同 Linux 发行版之间 GLIBC 版本不兼容以及架构差异的问题。

## 1. 环境准备

- **Rust 工具链**：已安装 `rustup` 及相应的 `nightly` 或 `stable` 版本。
- **Zig**：推荐版本 `0.15.2` 或更高。
- **cargo-zigbuild**：安装命令：
  ```bash
  cargo install cargo-zigbuild
  ```

## 2. 编译命令

在项目根目录下执行以下命令进行交叉编译：

```bash
cargo zigbuild --release --bin web_server --features="web_server" --target x86_64-unknown-linux-gnu
```

### 参数说明：
- `--release`：构建优化后的发行版本。
- `--bin web_server`：指定编译 `web_server` 二进制文件。
- `--features="web_server"`：启用 `web_server` 相关特性（如 Token 认证等）。
- `--target x86_64-unknown-linux-gnu`：指定目标平台为 Linux x86_64。

## 3. 编译产物

构建完成后，二进制文件位于：
`target/x86_64-unknown-linux-gnu/release/web_server`

## 4. 部署与运行

### 4.1 传输文件
使用 `scp` 将二进制文件传输至目标服务器：
```bash
scp target/x86_64-unknown-linux-gnu/release/web_server root@server_ip:/root/
```

### 4.2 赋予执行权限
在服务器上运行：
```bash
chmod +x /root/web_server
```

### 4.3 启动服务
通常配合服务启动脚本运行，例如：
```bash
/root/run_web.sh
```

## 5. 常见问题

- **GLIBC 版本问题**：`cargo-zigbuild` 默认会选择合适的 GLIBC 版本。如果服务器版本极低，可以通过 `--target x86_64-unknown-linux-gnu.2.17` 明确指定 GLIBC 版本（如 2.17）。
