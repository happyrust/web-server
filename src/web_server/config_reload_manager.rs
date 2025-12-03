//! 配置热重载管理器
//!
//! 提供配置的运行时重载功能，区分可热更新和需重启的配置项

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::net::{IpAddr, UdpSocket};
use std::sync::Arc;
use tokio::sync::RwLock;
use toml;

use config as cfg;

/// 可热重载的配置项
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HotReloadableConfig {
    /// MQTT 配置
    pub mqtt_host: String,
    pub mqtt_port: u16,

    /// 位置信息
    pub location: String,
    pub location_dbs: Vec<u32>,

    /// 文件服务器配置
    pub file_server_host: String,
    pub server_release_ip: String,

    /// 模型生成参数（部分可热更新）
    pub mesh_tol_ratio: f32,
    pub gen_spatial_tree: bool,

    /// 同步配置
    pub total_sync: bool,
    pub incr_sync: bool,
    pub sync_live: bool,
}

/// 需要重启的配置项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticConfig {
    /// 项目设置（需要重启）
    pub project_path: String,
    pub included_projects: Vec<String>,
    pub project_name: String,
    pub project_code: String,
    pub module: String,

    /// 数据库连接（需要重启）
    pub ip: String,
    pub user: String,
    pub password: String,
    pub port: String,

    /// 模型生成核心配置（需要重启）
    pub gen_model: bool,
    pub gen_mesh: bool,
    pub apply_boolean_operation: bool,
}

/// 配置变更事件
#[derive(Debug, Clone)]
pub struct ConfigChangeEvent {
    /// 热更新的变更键
    pub hot_changed_keys: HashSet<String>,
    /// 需要重启的变更键
    pub static_changed_keys: HashSet<String>,
    /// 是否需要重启
    pub requires_restart: bool,
    /// 事件时间
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// 配置重载管理器
pub struct ConfigReloadManager {
    /// 可热重载的配置
    hot_config: Arc<RwLock<HotReloadableConfig>>,

    /// 静态配置（只读）
    static_config: Arc<RwLock<StaticConfig>>,

    /// 配置变更监听器
    listeners: Arc<RwLock<Vec<Box<dyn Fn(ConfigChangeEvent) + Send + Sync>>>>,
}

impl ConfigReloadManager {
    /// 创建新的配置管理器
    pub fn new() -> Result<Self> {
        let (hot_config, static_config) = Self::load_from_file("DbOption.toml")?;

        Ok(Self {
            hot_config: Arc::new(RwLock::new(hot_config)),
            static_config: Arc::new(RwLock::new(static_config)),
            listeners: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// 从文件加载配置（直接解析 DbOption.toml，绕过全局缓存）
    fn load_from_file(path: &str) -> Result<(HotReloadableConfig, StaticConfig)> {
        // 使用 config crate 解析 TOML
        let cfg = cfg::Config::builder()
            .add_source(cfg::File::with_name(path.trim_end_matches(".toml")))
            .build()
            .with_context(|| format!("无法加载配置文件: {}", path))?;

        // 反序列化为 aios_core::options::DbOption
        let db_option: aios_core::options::DbOption =
            cfg.try_deserialize().context("无法反序列化 DbOption")?;

        // 处理 file_server_host 默认值
        let file_server_host = if db_option.file_server_host.is_empty() {
            // 从配置文件读取 web_server_host 和 web_server_port
            let config_file = format!("{}.toml", path.trim_end_matches(".toml"));
            let (server_ip, server_port) = if let Ok(content) = std::fs::read_to_string(&config_file) {
                if let Ok(toml_value) = content.parse::<toml::Value>() {
                    let web_host = toml_value
                        .get("web_server_host")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "0.0.0.0".to_string());
                    let web_port = toml_value
                        .get("web_server_port")
                        .and_then(|v| v.as_integer())
                        .map(|p| p as u16)
                        .unwrap_or(8080);
                    
                    // 如果 web_server_host 是 0.0.0.0，则获取本机 IP
                    let ip = if web_host == "0.0.0.0" {
                        Self::get_local_ip_via_udp().unwrap_or_else(|_| "127.0.0.1".to_string())
                    } else {
                        web_host
                    };
                    (ip, web_port)
                } else {
                    // 解析失败，使用默认值
                    (Self::get_local_ip_via_udp().unwrap_or_else(|_| "127.0.0.1".to_string()), 8080)
                }
            } else {
                // 文件读取失败，使用默认值
                (Self::get_local_ip_via_udp().unwrap_or_else(|_| "127.0.0.1".to_string()), 8080)
            };
            
            format!("http://{}:{}/assets/archives", server_ip, server_port)
        } else {
            db_option.file_server_host.clone()
        };

        let hot_config = HotReloadableConfig {
            mqtt_host: db_option.mqtt_host.clone(),
            mqtt_port: db_option.mqtt_port,
            location: db_option.location.clone(),
            location_dbs: db_option.location_dbs.clone().unwrap_or_default(),
            file_server_host,
            server_release_ip: db_option.server_release_ip.clone(),
            mesh_tol_ratio: db_option.mesh_tol_ratio.unwrap_or(3.0),
            gen_spatial_tree: db_option.gen_spatial_tree,
            total_sync: db_option.total_sync,
            incr_sync: db_option.incr_sync,
            sync_live: db_option.sync_live.unwrap_or(false),
        };

        let static_config = StaticConfig {
            project_path: db_option.project_path.clone(),
            included_projects: db_option.included_projects.clone(),
            project_name: db_option.project_name.clone(),
            project_code: db_option.project_code.clone(),
            module: db_option.module.clone(),
            ip: db_option.ip.clone(),
            user: db_option.user.clone(),
            password: db_option.password.clone(),
            port: db_option.port.clone(),
            gen_model: db_option.gen_model,
            gen_mesh: db_option.gen_mesh,
            apply_boolean_operation: db_option.apply_boolean_operation,
        };

        Ok((hot_config, static_config))
    }

    /// 获取可热重载的配置
    pub async fn get_hot_config(&self) -> HotReloadableConfig {
        self.hot_config.read().await.clone()
    }

    /// 获取静态配置
    pub async fn get_static_config(&self) -> StaticConfig {
        self.static_config.read().await.clone()
    }

    /// 重载配置（只重载可热更新的部分）
    pub async fn reload_hot_config(&self) -> Result<ConfigChangeEvent> {
        let (new_hot_config, new_static_config) = Self::load_from_file("DbOption.toml")?;

        // 检测变更
        let old_hot_config = self.hot_config.read().await.clone();
        let old_static_config = self.static_config.read().await.clone();
        let hot_changed_keys = self.detect_hot_changes(&old_hot_config, &new_hot_config);
        let static_changed_keys =
            self.detect_static_changes(&old_static_config, &new_static_config);

        // 更新热重载配置和静态配置快照
        *self.hot_config.write().await = new_hot_config;
        *self.static_config.write().await = new_static_config;

        // 创建变更事件
        let event = ConfigChangeEvent {
            hot_changed_keys: hot_changed_keys.clone(),
            static_changed_keys: static_changed_keys.clone(),
            requires_restart: !static_changed_keys.is_empty(),
            timestamp: chrono::Utc::now(),
        };

        // 通知监听器
        self.notify_listeners(event.clone()).await;

        Ok(event)
    }

    /// 检测热重载配置的变更
    fn detect_hot_changes(
        &self,
        old: &HotReloadableConfig,
        new: &HotReloadableConfig,
    ) -> HashSet<String> {
        let mut changes = HashSet::new();

        if old.mqtt_host != new.mqtt_host {
            changes.insert("mqtt_host".to_string());
        }
        if old.mqtt_port != new.mqtt_port {
            changes.insert("mqtt_port".to_string());
        }
        if old.location != new.location {
            changes.insert("location".to_string());
        }
        if old.location_dbs != new.location_dbs {
            changes.insert("location_dbs".to_string());
        }
        if old.file_server_host != new.file_server_host {
            changes.insert("file_server_host".to_string());
        }
        if old.server_release_ip != new.server_release_ip {
            changes.insert("server_release_ip".to_string());
        }
        if (old.mesh_tol_ratio - new.mesh_tol_ratio).abs() > f32::EPSILON {
            changes.insert("mesh_tol_ratio".to_string());
        }
        if old.gen_spatial_tree != new.gen_spatial_tree {
            changes.insert("gen_spatial_tree".to_string());
        }
        if old.total_sync != new.total_sync {
            changes.insert("total_sync".to_string());
        }
        if old.incr_sync != new.incr_sync {
            changes.insert("incr_sync".to_string());
        }
        if old.sync_live != new.sync_live {
            changes.insert("sync_live".to_string());
        }

        changes
    }

    /// 检测静态配置的变更
    fn detect_static_changes(&self, old: &StaticConfig, new: &StaticConfig) -> HashSet<String> {
        let mut changes = HashSet::new();

        if old.project_path != new.project_path {
            changes.insert("project_path".to_string());
        }
        if old.included_projects != new.included_projects {
            changes.insert("included_projects".to_string());
        }
        if old.project_name != new.project_name {
            changes.insert("project_name".to_string());
        }
        if old.project_code != new.project_code {
            changes.insert("project_code".to_string());
        }
        if old.module != new.module {
            changes.insert("module".to_string());
        }
        if old.ip != new.ip {
            changes.insert("ip".to_string());
        }
        if old.user != new.user {
            changes.insert("user".to_string());
        }
        if old.password != new.password {
            changes.insert("password".to_string());
        }
        if old.port != new.port {
            changes.insert("port".to_string());
        }
        if old.gen_model != new.gen_model {
            changes.insert("gen_model".to_string());
        }
        if old.gen_mesh != new.gen_mesh {
            changes.insert("gen_mesh".to_string());
        }
        if old.apply_boolean_operation != new.apply_boolean_operation {
            changes.insert("apply_boolean_operation".to_string());
        }

        changes
    }

    /// 通知所有监听器
    async fn notify_listeners(&self, event: ConfigChangeEvent) {
        let listeners = self.listeners.read().await;
        for listener in listeners.iter() {
            listener(event.clone());
        }
    }

    /// 添加配置变更监听器
    pub async fn add_listener<F>(&self, listener: F)
    where
        F: Fn(ConfigChangeEvent) + Send + Sync + 'static,
    {
        self.listeners.write().await.push(Box::new(listener));
    }

    /// 通过UdpSocket获取本机IP地址
    fn get_local_ip_via_udp() -> Result<String, std::io::Error> {
        // 连接到一个外部地址（不需要实际连接成功）
        // 这个方法会返回用于发送数据包的网络接口的IP地址
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.connect("8.8.8.8:80")?;
        let local_addr = socket.local_addr()?;

        if let IpAddr::V4(ipv4) = local_addr.ip() {
            Ok(ipv4.to_string())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "无法获取IPv4地址",
            ))
        }
    }
}
