#Requires -Version 5.1
<#
.SYNOPSIS
  将 Rust（rustup + stable）安装到 D 盘，并写入当前用户环境变量。

.DESCRIPTION
  - RUSTUP_HOME = D:\Rust\rustup
  - CARGO_HOME  = D:\Rust\cargo
  - User PATH 前置 D:\Rust\cargo\bin
  若曾用默认路径安装，请先卸载 winget 包并删除 %USERPROFILE%\.rustup / .cargo。

  需已安装 MSVC 生成工具（Visual Studio Build Tools）以使用 x86_64-pc-windows-msvc。
#>

$ErrorActionPreference = "Stop"
$RustRoot = "D:\Rust"
$RustupHome = Join-Path $RustRoot "rustup"
$CargoHome = Join-Path $RustRoot "cargo"
$CargoBin = Join-Path $CargoHome "bin"

New-Item -ItemType Directory -Force -Path $RustupHome, $CargoHome | Out-Null

# 从 User PATH 去掉旧的 %USERPROFILE%\.cargo\bin
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
$parts = $userPath -split ';' | Where-Object {
    $_ -and ($_ -notmatch '\\.cargo\\bin') -and ($_ -notmatch '\\.rustup\\')
}
$newPath = ($parts | Where-Object { $_.Trim() -ne '' }) -join ';'
[Environment]::SetEnvironmentVariable("Path", $newPath, "User")

[Environment]::SetEnvironmentVariable("RUSTUP_HOME", $RustupHome, "User")
[Environment]::SetEnvironmentVariable("CARGO_HOME", $CargoHome, "User")

$u2 = [Environment]::GetEnvironmentVariable("Path", "User")
if ($u2 -notmatch [regex]::Escape($CargoBin)) {
    [Environment]::SetEnvironmentVariable("Path", "$CargoBin;$u2", "User")
}

$env:RUSTUP_HOME = $RustupHome
$env:CARGO_HOME = $CargoHome
$env:Path = "$CargoBin;" + [Environment]::GetEnvironmentVariable("Path", "User") + ";" + [Environment]::GetEnvironmentVariable("Path", "Machine")

$init = Join-Path $env:TEMP "rustup-init.exe"
Invoke-WebRequest -Uri "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe" -OutFile $init -UseBasicParsing
& $init -y

Write-Host "完成。请重新打开终端，或执行: `$env:Path = [Environment]::GetEnvironmentVariable('Path','User') + ';' + [Environment]::GetEnvironmentVariable('Path','Machine')"
& (Join-Path $CargoBin "cargo.exe") --version
& (Join-Path $CargoBin "rustc.exe") --version
