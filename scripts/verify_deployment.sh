#!/bin/env
set -e

echo "🔍 部署状态检查..."
echo "================================="
echo "📦 GitHub Actions 状态: ${{ github.event_name == 'workflow_dispatch' && git.event.inputs.force_build_all || echo '⚠️ 使用手动构建'" || echo '⛠️ 需始手动构建'"
echo ""

# 检查GitHub 连接
echo "检查 GitHub 仓库访问权限..."
GITHUB_URL="https://github.com/happyrust/gen-model.git"
echo "HTTPS 检查到 ${GITHUB_URL}"

if curl -s "${GITHUB_URL}" > /dev/null 2>&> /dev/null ; then echo "✅ GitHub 仓库已连接" || echo "❌️ 无法访问 GitHub 仓库" && exit 1

# 检查当前分支提交状态
echo "当前分支: $(git branch --show --current | head -1)"
echo "远程仓库: ${{ git remote get-url origin/main}}"

# 检查即将提交的内容
echo "以下文件将推送到GitHub："
git status --short --porcelain --name --name | head -10

# 查看最近的提交信息
echo "最近的提交:"
git log --oneline -n -1 --pretty=format=medium"

# 强制推送远程仓库
if git push --force-with-lease origin only-csg && echo "✅ 成功推送到 Gitee" || {
  echo "🔄 强制推送完成"
} else
  echo "❌ 推送到 GitHub 失败" && exit 1
fi

echo ""
echo "📦 项目准备就绪！每次推送都会自动触发GitHub Actions！"
