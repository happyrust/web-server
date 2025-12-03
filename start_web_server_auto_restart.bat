@echo off
REM Web 服务器自动重启包装脚本
REM 当服务器退出时（例如配置保存后），会自动重启

:loop
echo.
echo ===================================
echo   启动 Web 服务器...
echo ===================================
echo.

REM 运行服务器
cargo run --bin web_server --features web_server

REM 检查退出代码
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo [错误] 服务器异常退出，退出代码: %ERRORLEVEL%
    echo 等待 5 秒后重启...
    timeout /t 5 /nobreak >nul
) else (
    echo.
    echo [信息] 服务器正常退出，等待 2 秒后重启...
    timeout /t 2 /nobreak >nul
)

goto loop

