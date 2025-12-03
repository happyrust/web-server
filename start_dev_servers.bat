@echo off
REM 增量更新开发环境启动脚本
REM 同时启动前端和后端服务器

echo.
echo ===================================
echo   增量更新开发环境启动脚本
echo ===================================
echo.

REM 检查是否在正确的目录
if not exist "frontend\package.json" (
    echo [错误] 请在 web-server 根目录下运行此脚本！
    pause
    exit /b 1
)

REM 检查是否已安装前端依赖
if not exist "frontend\node_modules" (
    echo [提示] 正在安装前端依赖...
    cd frontend
    call npm install
    cd ..
    echo.
)

echo [1/3] 启动前端开发服务器 (Vite)...
start "Frontend Dev Server" cmd /k "cd frontend && npm run dev"
timeout /t 2 /nobreak >nul

echo [2/3] 启动后端 Web 服务器 (Rust)...
start "Backend Web Server" cmd /k "cargo run --bin web_server --features web_server"
timeout /t 2 /nobreak >nul

echo [3/3] 等待服务器启动...
timeout /t 8 /nobreak >nul

echo.
echo ===================================
echo   服务器启动完成！
echo ===================================
echo.
echo 前端地址: http://localhost:3000
echo 后端地址: http://localhost:8080
echo WebSocket: ws://localhost:8080/ws/tasks
echo 测试页面: http://localhost:8080/test_websocket.html
echo.
echo 按任意键打开浏览器...
pause >nul

REM 打开前端界面
start http://localhost:3000

REM 打开测试页面
start http://localhost:8080/test_websocket.html

echo.
echo [完成] 浏览器已打开
echo.
echo 提示:
echo - 修改前端代码会自动热重载
echo - 修改后端代码需要手动重启服务器
echo - 关闭此窗口不会停止服务器
echo - 请手动关闭前端和后端的命令行窗口以停止服务
echo.
pause
