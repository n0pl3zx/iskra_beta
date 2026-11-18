@echo off
chcp 65001 >nul
echo Запускаю сервер «Искры» (без окна клиента)...
"%~dp0Iskra.exe" --server
pause
