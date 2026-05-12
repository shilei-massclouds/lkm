@echo off
setlocal

set "SCRIPT_DIR=%~dp0"
set "PYVERI_ROOT=%SCRIPT_DIR%.."

if defined PYTHONPATH (
    set "PYTHONPATH=%PYVERI_ROOT%\src;%PYTHONPATH%"
) else (
    set "PYTHONPATH=%PYVERI_ROOT%\src"
)

if not defined PYTHON set "PYTHON=python"
"%PYTHON%" -m pyveri %*
exit /b %ERRORLEVEL%
