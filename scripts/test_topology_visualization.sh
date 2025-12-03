#!/bin/bash

# 拓扑可视化测试脚本
# 用于验证拓扑 API 和前端显示是否正常

set -e

echo[object Object]测试脚本"
echo "================================"

# 检查 Web 服务器是否运行
echo ""
echo "1️⃣ 检查 Web 服务器状态..."
if curl -s http://localhost:8080/api/health > /dev/null 2>&1; then
    echo "✅ Web 服务器正在运行"
else
    echo "❌ Web 服务器未运行，请先启动："
    echo "   cargo run --bin web_server --features web_server"
    exit 1
fi

# 测试拓扑 API
echo ""
echo "2️⃣ 测试拓扑配置 API..."
TOPOLOGY_RESPONSE=$(curl -s http://localhost:8080/api/remote-sync/topology)
echo "$TOPOLOGY_RESPONSE" | jq '.' > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "✅ 拓扑 API 响应正常"
    
    # 提取统计信息
    ENV_COUNT=$(echo "$TOPOLOGY_RESPONSE" | jq '.data.environments | length')
    SITE_COUNT=$(echo "$TOPOLOGY_RESPONSE" | jq '.data.sites | length')
    CONN_COUNT=$(echo "$TOPOLOGY_RESPONSE" | jq '.data.connections | length')
    
    ec[object Object]境节点: $ENV_COUNT 个"
    echo[object Object]点: $SITE_COUNT 个"
    echo "[object Object]$CONN_COUNT 条"
    
    if [ "$ENV_COUNT" -eq 0 ] && [ "$SITE_COUNT" -eq 0 ]; then
        echo "   ⚠️  警告: 拓扑配置为空，请先配置环境和站点"
    fi
else
    echo "❌ 拓扑 API 响应异常"
    echo "$TOPOLOGY_RESPONSE"
    exit 1
fi

# 测试 MQTT 节点 API
echo ""
echo "3️⃣ 测试 MQTT 节点 API..."
NODES_RESPONSE=$(curl -s http://localhost:8080/api/mqtt/nodes)
echo "$NODES_RESPONSE" | jq '.' > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "✅ MQTT 节点 API 响应正常"
    
    # 提取统计信息
    NODE_COUNT=$(echo "$NODES_RESPONSE" | jq '.nodes | length')
    ONLINE_COUNT=$(echo "$NODES_RESPONSE" | jq '[.nodes[] | select(.is_online == true)] | length')
    MASTER_COUNT=$(echo "$NODES_RESPONSE" | jq '[.nodes[] | select(.is_master_node == true)] | length')
    
    echo "   📊 节点总数: $NODE_COUNT 个"
    echo[object Object]在线节点: $ONLINE_COUNT 个"
    echo "   📊 主节点: $MASTER_COUNT 个"
    
    if [ "$NODE_COUNT" -eq 0 ]; then
        echo "   ⚠️  警告: 没有 MQTT 节点，请确保节点已启动并连接"
    fi
else
    echo "❌ MQTT 节点 API 响应异常"
    echo "$NODES_RESPONSE"
    exit 1
fi

# 测试消息 API
echo ""
echo "4️⃣ 测试 MQTT 消息 API..."
MESSAGES_RESPONSE=$(curl -s "http://localhost:8080/api/mqtt/messages?limit=10")
echo "$MESSAGES_RESPONSE" | jq '.' > /dev/null 2>&1
if [ $? -eq 0 ]; then
    echo "✅ MQTT 消息 API 响应正常"
    
    # 提取统计信息
    MSG_COUNT=$(echo "$MESSAGES_RESPONSE" | jq '.messages | length')
    echo "   📊 最近消息: $MSG_COUNT 条"
else
    echo "❌ MQTT 消息 API 响应异常"
    echo "$MESSAGES_RESPONSE"
    exit 1
fi

# 检查前端文件
echo ""
echo "5️⃣ 检查前端文件..."
if [ -f "frontend/src/components/views/TopologyVisualization.vue" ]; then
    echo "✅ 拓扑可视化组件存在"
    
    # 检查是否使用正确的 API 路径
    if grep -q "/api/remote-sync/topology" frontend/src/components/views/TopologyVisualization.vue; then
        echo "✅ API 路径正确"
    else
        echo "⚠️  警告: API 路径可能不正确"
    fi
else
    echo "❌ 拓扑可视化组件不存在"
    exit 1
fi

# 总结
echo ""
echo "================================"
echo "✅ 测试完成！"
echo ""
echo "📝 下一步操作："
echo "   1. 打开浏览器: http://localhost:8080"
echo "   2. 点击左侧导航 '拓扑可视化'"
echo "   3. 按 F12 打开控制台查看日志"
echo "   4. 验证所有节点和连接是否正确显示"
echo ""
echo "🔍 预期看到："
echo "   - 主节点在中心（紫色圆圈）"
echo "   - 从节点环绕外圈（绿色=在线，红色=离线）"
echo "   - 连接线（绿色=已订阅，蓝色=在线，灰色=离线）"
echo "   - 点击节点显示详情面板"
echo ""

