@echo off
chcp 65001 >nul
setlocal
if "%CARGO_HOME%"=="" set CARGO_HOME=%USERPROFILE%\.cargo
if "%RUSTUP_HOME%"=="" set RUSTUP_HOME=%USERPROFILE%\.rustup
set CARGO_TARGET_DIR=%~dp0target
set PATH=%CARGO_HOME%\bin;%PATH%
echo Сборка релизной версии «Искры»...
cargo build --release --manifest-path "%~dp0Cargo.toml"
if errorlevel 1 (
  echo.
  echo Сборка не удалась.
  pause
  exit /b 1
)
copy /y "%~dp0target\release\iskra.exe" "%~dp0Iskra.exe" >nul
echo Готово: %~dp0Iskra.exe
pause
