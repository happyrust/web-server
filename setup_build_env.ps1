Write-Host "Setting up Windows build environment for web-server project..." -ForegroundColor Green

# Install required tools using winget
Write-Host "Installing CMake..." -ForegroundColor Yellow
winget install --id Kitware.CMake --source winget

Write-Host "Installing NASM..." -ForegroundColor Yellow
winget install --id "NASM.NASM" --source winget

Write-Host "Installing LLVM..." -ForegroundColor Yellow
winget install --id LLVM.LLVM --source winget

# Set environment variables for current session
Write-Host "Setting environment variables..." -ForegroundColor Yellow
$env:AWS_LC_SYS_NO_ASM = "1"
$env:LIBCLANG_PATH = "D:\LLVM\bin"
$env:PATH = "C:\Program Files\CMake\bin;C:\Program Files\NASM;D:\LLVM\bin;" + $env:PATH

# Verify installation
Write-Host "`nVerifying tool installation..." -ForegroundColor Yellow

try {
    $cmakeVersion = cmake --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ CMake installed correctly" -ForegroundColor Green
    } else {
        Write-Host "✗ ERROR: CMake not found in PATH" -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Host "✗ ERROR: CMake not found" -ForegroundColor Red
    exit 1
}

try {
    $nasmVersion = nasm --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ NASM installed correctly" -ForegroundColor Green
    } else {
        Write-Host "✗ ERROR: NASM not found in PATH" -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Host "✗ ERROR: NASM not found" -ForegroundColor Red
    exit 1
}

try {
    $clangVersion = clang --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ clang installed correctly" -ForegroundColor Green
    } else {
        Write-Host "✗ ERROR: clang not found in PATH" -ForegroundColor Red
        exit 1
    }
} catch {
    Write-Host "✗ ERROR: clang not found" -ForegroundColor Red
    exit 1
}

Write-Host "`nBuild environment setup complete!" -ForegroundColor Green
Write-Host "You can now run: cargo check" -ForegroundColor Cyan
Write-Host "`nFor permanent environment setup, add these to your system environment variables:" -ForegroundColor Cyan
Write-Host "  AWS_LC_SYS_NO_ASM=1" -ForegroundColor White
Write-Host "  LIBCLANG_PATH=D:\LLVM\bin" -ForegroundColor White
Write-Host "  PATH=...C:\Program Files\CMake\bin;C:\Program Files\NASM;D:\LLVM\bin;...existing path..." -ForegroundColor White

# Keep window open
Write-Host "`nPress any key to continue..." -ForegroundColor Cyan
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
