#!/bin/bash
# Web Server 集成功能测试脚本

set -e

echo "=========================================="
echo "Web Server 集成功能测试"
echo "=========================================="
echo ""

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查依赖
echo "检查依赖..."
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ cargo 未安装${NC}"
    exit 1
fi
echo -e "${GREEN}✅ cargo 已安装${NC}"

if ! command -v curl &> /dev/null; then
    echo -e "${YELLOW}⚠️  curl 未安装，部分测试将跳过${NC}"
    CURL_AVAILABLE=false
else
    echo -e "${GREEN}✅ curl 已安装${NC}"
    CURL_AVAILABLE=true
fi

echo ""

# 编译项目
echo "=========================================="
echo "编译项目..."
echo "=========================================="
cargo build --bin egui_remote_sync --features gui --release

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ 编译成功${NC}"
else
    echo -e "${RED}❌ 编译失败${NC}"
    exit 1
fi

echo ""

# 提示用户
echo "=========================================="
echo "手动测试指南"
echo "=========================================="
echo ""
echo "请按照以下步骤进行测试："
echo ""
echo "1. 启动 GUI 应用："
echo "   ${YELLOW}cargo run --bin egui_remote_sync --features gui${NC}"
echo ""
echo "2. 在 GUI 中："
echo "   - 点击左侧导航栏的 'Web Server'"
echo "   - 点击 '▶️ 启动' 按钮"
echo "   - 观察服务器状态变为 '🟢 运行中'"
echo ""
echo "3. 在另一个终端运行以下命令测试："
echo ""

if [ "$CURL_AVAILABLE" = true ]; then
    echo "   ${YELLOW}# 测试健康检查${NC}"
    echo "   curl http://localhost:3000/health"
    echo ""
    echo "   ${YELLOW}# 发送多个请求${NC}"
    echo "   for i in {1..10}; do curl -s http://localhost:3000/health; done"
    echo ""
    echo "   ${YELLOW}# 测试 404${NC}"
    echo "   curl http://localhost:3000/nonexistent"
    echo ""
fi

echo "4. 在 GUI 中观察："
echo "   - 请求统计卡片更新"
echo "   - 请求日志表格显示记录"
echo "   - 性能统计更新"
echo ""
echo "5. 测试其他功能："
echo "   - 点击 '⏹️ 停止' 按钮"
echo "   - 点击 '🔄 重启' 按钮"
echo "   - 点击 '📤 导出配置' 按钮"
echo "   - 点击 '📥 导入配置' 按钮"
echo "   - 点击 '📱 生成二维码' 按钮"
echo ""

# 如果 curl 可用，提供自动化测试选项
if [ "$CURL_AVAILABLE" = true ]; then
    echo "=========================================="
    echo "自动化测试（需要 GUI 已启动）"
    echo "=========================================="
    echo ""
    read -p "是否运行自动化测试？(y/n) " -n 1 -r
    echo ""
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo ""
        echo "等待 GUI 启动..."
        echo "请在另一个终端启动 GUI，然后按回车继续..."
        read
        
        echo ""
        echo "开始自动化测试..."
        echo ""
        
        # 测试健康检查
        echo -n "测试健康检查端点... "
        if curl -f -s http://localhost:3000/health > /dev/null 2>&1; then
            echo -e "${GREEN}✅ 通过${NC}"
        else
            echo -e "${RED}❌ 失败${NC}"
            echo "提示: 确保 GUI 中的 Web Server 已启动"
        fi
        
        # 发送多个请求
        echo -n "发送 10 个测试请求... "
        SUCCESS_COUNT=0
        for i in {1..10}; do
            if curl -f -s http://localhost:3000/health > /dev/null 2>&1; then
                ((SUCCESS_COUNT++))
            fi
        done
        
        if [ $SUCCESS_COUNT -eq 10 ]; then
            echo -e "${GREEN}✅ 通过 (10/10)${NC}"
        else
            echo -e "${YELLOW}⚠️  部分成功 ($SUCCESS_COUNT/10)${NC}"
        fi
        
        # 测试 404
        echo -n "测试 404 响应... "
        HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/nonexistent)
        if [ "$HTTP_CODE" = "404" ]; then
            echo -e "${GREEN}✅ 通过${NC}"
        else
            echo -e "${YELLOW}⚠️  返回码: $HTTP_CODE${NC}"
        fi
        
        echo ""
        echo "自动化测试完成！"
        echo ""
        echo "请在 GUI 中检查："
        echo "  - 请求统计显示至少 11 个请求"
        echo "  - 请求日志显示所有请求记录"
        echo "  - 至少有一个失败请求（404）"
    fi
fi

echo ""
echo "=========================================="
echo "测试完成"
echo "=========================================="
echo ""
echo "详细测试清单请参考："
echo "  .kiro/specs/egui-web-server-integration/TESTING_CHECKLIST.md"
echo ""
echo "实现文档请参考："
echo "  .kiro/specs/egui-web-server-integration/IMPLEMENTATION_SUMMARY.md"
echo ""
echo "快速入门请参考："
echo "  .kiro/specs/egui-web-server-integration/QUICKSTART.md"
echo ""
