#!/bin/bash
# 快速测试 Web Server 功能

echo "=========================================="
echo "Web Server 快速测试"
echo "=========================================="
echo ""

# 等待用户在 GUI 中启动服务器
echo "请在 GUI 中执行以下操作："
echo "1. 点击左侧导航栏的 'Web Server'"
echo "2. 点击 '▶️ 启动' 按钮"
echo "3. 确认状态显示为 '🟢 运行中'"
echo ""
echo "完成后按回车继续测试..."
read

echo ""
echo "开始测试..."
echo ""

# 测试 1: 健康检查
echo -n "测试 1: 健康检查端点... "
if curl -f -s http://localhost:3000/health > /dev/null 2>&1; then
    echo "✅ 通过"
else
    echo "❌ 失败 (确保服务器已启动)"
    exit 1
fi

# 测试 2: 发送多个请求
echo -n "测试 2: 发送 10 个请求... "
SUCCESS=0
for i in {1..10}; do
    if curl -f -s http://localhost:3000/health > /dev/null 2>&1; then
        ((SUCCESS++))
    fi
done
echo "✅ 完成 ($SUCCESS/10)"

# 测试 3: 测试 404
echo -n "测试 3: 测试 404 响应... "
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/nonexistent)
if [ "$HTTP_CODE" = "404" ]; then
    echo "✅ 通过 (状态码: $HTTP_CODE)"
else
    echo "⚠️  状态码: $HTTP_CODE"
fi

echo ""
echo "=========================================="
echo "测试完成！"
echo "=========================================="
echo ""
echo "请在 GUI 中检查："
echo "  ✓ 请求统计显示至少 11 个请求"
echo "  ✓ 成功请求数为 10"
echo "  ✓ 失败请求数为 1"
echo "  ✓ 请求日志表格显示所有请求"
echo "  ✓ 最后一条日志状态码为 404（红色）"
echo ""
