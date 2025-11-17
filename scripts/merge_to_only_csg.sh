#!/bin/bash
# 快速合并 egui-ui-dev 到 only-csg 分支

set -e

echo "=========================================="
echo "合并 egui-ui-dev 到 only-csg"
echo "=========================================="
echo ""

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查是否在正确的目录
WORKTREE_PATH="/Volumes/DPC/work/plant-code/gen-model-fork"

if [ ! -d "$WORKTREE_PATH" ]; then
    echo -e "${RED}❌ 工作树目录不存在: $WORKTREE_PATH${NC}"
    exit 1
fi

echo -e "${GREEN}✅ 找到工作树目录${NC}"
echo ""

# 切换到工作树目录
cd "$WORKTREE_PATH"

echo "当前目录: $(pwd)"
echo ""

# 检查当前分支
CURRENT_BRANCH=$(git branch --show-current)
echo "当前分支: $CURRENT_BRANCH"

if [ "$CURRENT_BRANCH" != "only-csg" ]; then
    echo -e "${YELLOW}⚠️  当前不在 only-csg 分支${NC}"
    read -p "是否切换到 only-csg 分支? (y/n) " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git checkout only-csg
    else
        echo "已取消"
        exit 1
    fi
fi

echo ""
echo "=========================================="
echo "拉取最新代码"
echo "=========================================="
echo ""

# 拉取最新的 egui-ui-dev 分支
git fetch origin egui-ui-dev

echo ""
echo "=========================================="
echo "开始合并"
echo "=========================================="
echo ""

# 显示将要合并的提交
echo "将要合并的提交:"
git log --oneline only-csg..origin/egui-ui-dev | head -5
echo ""

read -p "确认合并? (y/n) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "已取消"
    exit 1
fi

# 合并
if git merge origin/egui-ui-dev --no-edit; then
    echo ""
    echo -e "${GREEN}✅ 合并成功！${NC}"
    echo ""
    
    # 显示合并后的状态
    echo "合并后的状态:"
    git status
    echo ""
    
    read -p "是否推送到远程? (y/n) " -n 1 -r
    echo ""
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        git push origin only-csg
        echo ""
        echo -e "${GREEN}✅ 推送成功！${NC}"
    else
        echo -e "${YELLOW}⚠️  未推送，请手动执行: git push origin only-csg${NC}"
    fi
else
    echo ""
    echo -e "${RED}❌ 合并失败，存在冲突${NC}"
    echo ""
    echo "冲突文件:"
    git diff --name-only --diff-filter=U
    echo ""
    echo "请手动解决冲突，然后执行:"
    echo "  git add <冲突文件>"
    echo "  git commit"
    echo "  git push origin only-csg"
    exit 1
fi

echo ""
echo "=========================================="
echo "验证编译"
echo "=========================================="
echo ""

read -p "是否验证编译? (y/n) " -n 1 -r
echo ""

if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "开始编译..."
    if cargo check --features gui; then
        echo ""
        echo -e "${GREEN}✅ 编译成功！${NC}"
    else
        echo ""
        echo -e "${RED}❌ 编译失败${NC}"
        echo "请检查代码并修复错误"
        exit 1
    fi
fi

echo ""
echo "=========================================="
echo "合并完成！"
echo "=========================================="
echo ""
echo "下一步:"
echo "  1. 测试功能: cargo run --bin egui_remote_sync --features gui"
echo "  2. 查看文档: .kiro/specs/egui-web-server-integration/README.md"
echo ""
