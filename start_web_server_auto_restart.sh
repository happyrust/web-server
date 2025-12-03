#!/bin/bash
# Web 服务器自动重启包装脚本
# 当服务器退出时（例如配置保存后），会自动重启

while true; do
    echo ""
    echo "==================================="
    echo "  启动 Web 服务器..."
    echo "==================================="
    echo ""
    
    # 运行服务器
    cargo run --bin web_server --features web_server
    
    # 检查退出代码
    exit_code=$?
    if [ $exit_code -ne 0 ]; then
        echo ""
        echo "[错误] 服务器异常退出，退出代码: $exit_code"
        echo "等待 5 秒后重启..."
        sleep 5
    else
        echo ""
        echo "[信息] 服务器正常退出，等待 2 秒后重启..."
        sleep 2
    fi
done

