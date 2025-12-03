@echo off
REM 启动 MQTT 服务器 (rumqttd)
REM 使用测试目录中的配置文件

echo ========================================
echo 启动 MQTT 服务器
echo ========================================
echo.

cd "%~dp0remote-test-dir\test-real"

echo 配置文件: rumqttd.toml
echo MQTT 端口: 1883
echo 控制台端口: 18083
echo.
echo 按 Ctrl+C 停止服务器
echo.

rumqttd.exe -c rumqttd.toml -vv
