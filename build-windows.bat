@echo off
REM Builds release .exe files for every wisp-web tool. Run this from the
REM wisp-web folder (double-click it in Explorer, or run it in PowerShell/cmd).

echo Building wisp-web (release mode)...
cargo build --release --workspace
if %errorlevel% neq 0 (
    echo.
    echo Build failed - see errors above.
    pause
    exit /b 1
)

echo.
echo Done. Executables are in target\release\:
echo   wisp_browser.exe   (the GUI browser)
echo   wisp-site.exe      (scaffold/serve sites)
echo   wisp-search.exe    (crawl/serve search)
echo   server.exe         (bare wisp-protocol server)
echo   client.exe         (bare wisp-protocol client)
echo.
pause
