@echo off
setlocal

rem 可选：工具链目录。优先级：GH_FINDER_TOOLS_DIR > 仓库同级的 tools 目录。
rem 未找到时直接使用系统 PATH 里的 Rust / Tauri CLI。
if defined GH_FINDER_TOOLS_DIR set "TOOLS=%GH_FINDER_TOOLS_DIR%"
if not defined TOOLS if exist "%~dp0..\tools\rust\rustup" for %%I in ("%~dp0..\tools") do set "TOOLS=%%~fI"

if defined TOOLS if exist "%TOOLS%\rust\rustup" (
  set "RUSTUP_HOME=%TOOLS%\rust\rustup"
  set "CARGO_HOME=%TOOLS%\rust\cargo"
  if exist "%TOOLS%\rust\rustup\toolchains\stable-x86_64-pc-windows-gnu" (
    set "RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-gnu"
  )
  set "PATH=%TOOLS%\mingw\mingw64\bin;%TOOLS%\rust\cargo\bin;%TOOLS%\tauri-cli;%PATH%"
)

cd /d "%~dp0"
python export_lexicon.py --check
if errorlevel 1 exit /b 1
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
