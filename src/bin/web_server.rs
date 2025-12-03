use aios_database::web_server::start_web_server_with_config;
use clap::{Arg, Command};
use std::net::{IpAddr, UdpSocket};
use toml;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let matches = Command::new("aios-web-server")
        .version("0.1.3")
        .about("AIOS Web UI Server")
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .help("Path to the configuration file (Without extension)")
                .value_name("CONFIG_PATH")
                .default_value("DbOption"),
        )
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .help("Port number to listen on (default: 8080, or from PORT environment variable)")
                .value_name("PORT"),
        )
        .arg(
            Arg::new("host")
                .long("host")
                .help("Host address to bind to (default: 0.0.0.0, or from WEB_SERVER_HOST environment variable)")
                .value_name("HOST"),
        )
        .get_matches();

    // 获取配置文件路径
    let config_path = matches
        .get_one::<String>("config")
        .expect("default value ensures this exists");

    // 设置环境变量，让 rs-core 库使用正确的配置文件
    unsafe {
        std::env::set_var("DB_OPTION_FILE", config_path);
    }

    // 初始化日志，过滤掉 reqwest/hyper 的 debug 日志
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("debug,reqwest=warn,hyper=warn,hyper_util=warn")
    ).init();

    // 读取配置文件中的 web_server_host 和 web_server_port
    // 优先级：命令行参数 > 环境变量 > 配置文件 > 默认值
    let (config_host, config_port) = {
        // 先设置环境变量以便 get_db_option 能读取正确的配置文件
        unsafe {
            std::env::set_var("DB_OPTION_FILE", config_path);
        }
        
        // 尝试从配置文件读取
        let _opt = aios_core::get_db_option();
        // 使用 toml 直接读取，因为 DbOption 结构体可能没有这些字段
        let config_file = format!("{}.toml", config_path);
        let (host, port) = if let Ok(content) = std::fs::read_to_string(&config_file) {
            if let Ok(toml_value) = content.parse::<toml::Value>() {
                let host = toml_value
                    .get("web_server_host")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "0.0.0.0".to_string());
                let port = toml_value
                    .get("web_server_port")
                    .and_then(|v| v.as_integer())
                    .map(|p| p as u16)
                    .unwrap_or(8080);
                (Some(host), Some(port))
            } else {
                (Some("0.0.0.0".to_string()), Some(8080))
            }
        } else {
            (Some("0.0.0.0".to_string()), Some(8080))
        };
        (host, port)
    };

    // 端口优先级：命令行参数 > 环境变量 > 配置文件 > 默认值8080
    let port = if let Some(port_str) = matches.get_one::<String>("port") {
        port_str.parse::<u16>().unwrap_or_else(|_| {
            eprintln!("⚠️  警告: 无效的端口号 '{}'，使用默认端口 8080", port_str);
            8080
        })
    } else {
        std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .or(config_port)
            .unwrap_or(8080)
    };

    // Host 优先级：命令行参数 > 环境变量 > 配置文件 > 默认值0.0.0.0
    let host = if let Some(host_str) = matches.get_one::<String>("host") {
        host_str.clone()
    } else {
        std::env::var("WEB_SERVER_HOST")
            .ok()
            .or_else(|| config_host)
            .unwrap_or_else(|| "0.0.0.0".to_string())
    };

    // 根据绑定地址选择显示的IP
    let display_ip = if host == "0.0.0.0" {
        get_local_ip_via_udp().unwrap_or_else(|_| "127.0.0.1".to_string())
    } else {
        host.clone()
    };

    println!("🚀 正在启动 AIOS Web UI 服务器...");
    println!("📱 访问地址: http://{}:{}", display_ip, port);
    println!("⚙️  使用配置文件: {}.toml", config_path);
    println!("🔌 绑定地址: {}:{}", host, port);
    println!("💡 数据库服务由配置管理，根据需要启动");

    start_web_server_with_config(port, Some(config_path), Some(host)).await?;

    Ok(())
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
