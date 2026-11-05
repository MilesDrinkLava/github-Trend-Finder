@echo off
setlocal

rem 可选：如果本机工具链装在 E:\ClaudeCode\tools，自动切过去；
rem 其他环境直接使用系统 PATH 里的 Rust / Tauri CLI。
set "TOOLS=E:\ClaudeCode\tools"
if exist "%TOOLS%\rust\rustup" (
  set "RUSTUP_HOME=%TOOLS%\rust\rustup"
  set "CARGO_HOME=%TOOLS%\rust\cargo"
  set "RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-gnu"
  set "PATH=%TOOLS%\mingw\mingw64\bin;%TOOLS%\rust\cargo\bin;%TOOLS%\tauri-cli;%PATH%"
)

cd /d "%~dp0src-tauri"

where cargo-tauri.exe >nul 2>nul
if errorlevel 1 (
  cargo tauri build
) else (
  cargo-tauri.exe build
)

if errorlevel 1 (
  echo.
  echo 构建失败，请检查上面的错误信息。
  pause
  exit /b 1
)

echo.
echo 安装包已生成：%~dp0src-tauri\target\release\bundle\nsis\
pause
