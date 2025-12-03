@echo off
echo Setting up Windows build environment for web-server project...

REM Install required tools using winget
echo Installing CMake...
winget install --id Kitware.CMake --source winget

echo Installing NASM...
winget install --id "NASM.NASM" --source winget

echo Installing LLVM...
winget install --id LLVM.LLVM --source winget

REM Set environment variables for current session
echo Setting environment variables...
set AWS_LC_SYS_NO_ASM=1
set LIBCLANG_PATH=D:\LLVM\bin
set PATH=C:\Program Files\CMake\bin;C:\Program Files\NASM;D:\LLVM\bin;%PATH%

REM Verify installation
echo Verifying tool installation...
cmake --version
if %errorlevel% neq 0 (
    echo ERROR: CMake not found in PATH
    exit /b 1
)

nasm --version
if %errorlevel% neq 0 (
    echo ERROR: NASM not found in PATH
    exit /b 1
)

clang --version
if %errorlevel% neq 0 (
    echo ERROR: clang not found in PATH
    exit /b 1
)

echo.
echo Build environment setup complete!
echo You can now run: cargo check
echo.
echo For permanent environment setup, add these to your system environment variables:
echo   AWS_LC_SYS_NO_ASM=1
echo   LIBCLANG_PATH=D:\LLVM\bin
echo   PATH=...C:\Program Files\CMake\bin;C:\Program Files\NASM;D:\LLVM\bin;...existing path...
pause
