# Windows Build Environment Check Script
# This script checks if all required build dependencies are properly installed

Write-Host "Checking Windows build environment..." -ForegroundColor Green

$issues = @()

# Check CMake
Write-Host "`nChecking CMake..." -ForegroundColor Yellow
try {
    $cmakeVersion = cmake --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ CMake found: $($cmakeVersion[0].Split()[2])" -ForegroundColor Green
    } else {
        $issues += "CMake not found or not working"
        Write-Host "✗ CMake not found" -ForegroundColor Red
    }
} catch {
    $issues += "CMake not installed"
    Write-Host "✗ CMake not installed" -ForegroundColor Red
}

# Check NASM
Write-Host "`nChecking NASM..." -ForegroundColor Yellow
try {
    $nasmVersion = nasm --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ NASM found: $($nasmVersion[0].Split()[1])" -ForegroundColor Green
    } else {
        $issues += "NASM not found or not working"
        Write-Host "✗ NASM not found" -ForegroundColor Red
    }
} catch {
    $issues += "NASM not installed"
    Write-Host "✗ NASM not installed" -ForegroundColor Red
}

# Check clang/libclang
Write-Host "`nChecking clang..." -ForegroundColor Yellow
try {
    $clangVersion = clang --version 2>$null
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✓ clang found: $($clangVersion[0].Split()[-1])" -ForegroundColor Green
    } else {
        $issues += "clang not found or not working"
        Write-Host "✗ clang not found" -ForegroundColor Red
    }
} catch {
    $issues += "clang not installed"
    Write-Host "✗ clang not installed" -ForegroundColor Red
}

# Check environment variables
Write-Host "`nChecking environment variables..." -ForegroundColor Yellow

if ($env:AWS_LC_SYS_NO_ASM -eq "1") {
    Write-Host "✓ AWS_LC_SYS_NO_ASM=1 is set" -ForegroundColor Green
} else {
    $issues += "AWS_LC_SYS_NO_ASM not set to 1"
    Write-Host "✗ AWS_LC_SYS_NO_ASM not set (run: `$env:AWS_LC_SYS_NO_ASM = '1`)" -ForegroundColor Red
}

if ($env:LIBCLANG_PATH) {
    Write-Host "✓ LIBCLANG_PATH is set to: $env:LIBCLANG_PATH" -ForegroundColor Green
} else {
    $issues += "LIBCLANG_PATH not set"
    Write-Host "✗ LIBCLANG_PATH not set (run: `$env:LIBCLANG_PATH = 'D:\LLVM\bin`)" -ForegroundColor Red
}

# Test cargo check
Write-Host "`nTesting cargo check..." -ForegroundColor Yellow
$testResult = cargo check --quiet 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✓ cargo check passed successfully" -ForegroundColor Green
} else {
    $issues += "cargo check failed"
    Write-Host "✗ cargo check failed" -ForegroundColor Red
    Write-Host "Error details:" -ForegroundColor Red
    $errorLines = $testResult -split "`n" | Select-Object -First 5
    $errorLines | ForEach-Object { Write-Host "  $_" -ForegroundColor Gray }
}

# Summary
Write-Host "`n" + ("-"*50) -ForegroundColor Cyan
if ($issues.Count -eq 0) {
    Write-Host "✓ All build dependencies are properly configured!" -ForegroundColor Green
    Write-Host "You can compile the project successfully." -ForegroundColor Green
} else {
    Write-Host "✗ Found $($issues.Count) issue(s) that need to be resolved:" -ForegroundColor Red
    foreach ($issue in $issues) {
        Write-Host "  • $issue" -ForegroundColor Yellow
    }
    Write-Host "`nRun setup_build_env.ps1 to install missing dependencies." -ForegroundColor Cyan
}

Write-Host "`nPress any key to continue..." -ForegroundColor Cyan
$null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
