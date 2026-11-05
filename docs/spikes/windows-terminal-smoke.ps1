# Run in x64 PowerShell 7 on Windows with VS C++ x64 tools installed.
[CmdletBinding()]
param()
$ErrorActionPreference = 'Stop'
$spikeRoot = Join-Path ([IO.Path]::GetTempPath()) ('csp-wt-isolated-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $spikeRoot | Out-Null
Push-Location -LiteralPath $spikeRoot
try {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    $vs = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if (!$vs) { throw 'Visual Studio C++ x64 tools are required.' }
    $env:CSP_WT_VSDEV = Join-Path $vs 'Common7\Tools\VsDevCmd.bat'
    $terminalUrl = 'https://github.com/microsoft/terminal/releases/download/v1.24.12741.0/Microsoft.WindowsTerminal_1.24.12741.0_x64.zip'
    $detoursUrl = 'https://codeload.github.com/microsoft/Detours/zip/refs/tags/v4.0.1'
    Invoke-WebRequest $terminalUrl -OutFile terminal.zip
    Invoke-WebRequest $detoursUrl -OutFile detours.zip
    Expand-Archive -LiteralPath terminal.zip -DestinationPath $spikeRoot
    Expand-Archive -LiteralPath detours.zip -DestinationPath $spikeRoot
@'
#include <windows.h>
#include <shlobj.h>
#include <roapi.h>
#include <winstring.h>
#include <cstdio>
#include <detours.h>
#include <cwchar>

static decltype(&SHGetKnownFolderPath) real_folder = SHGetKnownFolderPath;
static decltype(&RoGetActivationFactory) real_factory = RoGetActivationFactory;

static HRESULT WINAPI isolated_folder(REFKNOWNFOLDERID id, DWORD flags, HANDLE token, PWSTR* result)
{
    const wchar_t* var = nullptr;
    if (IsEqualGUID(id, FOLDERID_LocalAppData)) var = L"CSP_WT_LOCAL";
    if (IsEqualGUID(id, FOLDERID_ProgramData)) var = L"CSP_WT_PROGRAM";
    if (!var) return real_folder(id, flags, token, result);
    wchar_t path[MAX_PATH]{};
    const auto len = GetEnvironmentVariableW(var, path, MAX_PATH);
    if (!len || len >= MAX_PATH) return E_FAIL;
    wchar_t log[MAX_PATH]{};
    if (GetEnvironmentVariableW(L"CSP_WT_HOOK_LOG", log, MAX_PATH)) {
        FILE* file = nullptr;
        if (!_wfopen_s(&file, log, L"a")) {
            fwprintf(file, L"%ls -> %ls\n", var, path);
            fclose(file);
        }
    }
    *result = static_cast<PWSTR>(CoTaskMemAlloc((len + 1) * sizeof(wchar_t)));
    if (!*result) return E_OUTOFMEMORY;
    wcscpy_s(*result, len + 1, path);
    return S_OK;
}

static HRESULT WINAPI isolated_factory(HSTRING name, REFIID iid, void** result)
{
    if (wcscmp(WindowsGetStringRawBuffer(name, nullptr),
               L"Windows.ApplicationModel.AppExtensions.AppExtensionCatalog") == 0)
        return REGDB_E_CLASSNOTREG;
    return real_factory(name, iid, result);
}

BOOL WINAPI DllMain(HINSTANCE, DWORD reason, LPVOID)
{
    if (DetourIsHelperProcess()) return TRUE;
    if (reason != DLL_PROCESS_ATTACH) return TRUE;
    DetourRestoreAfterWith();
    auto folderContract = LoadLibraryW(L"ext-ms-win-shell32-shellfolders-l1-1-0.dll");
    if (!folderContract) return FALSE;
    real_folder = reinterpret_cast<decltype(real_folder)>(GetProcAddress(folderContract, "SHGetKnownFolderPath"));
    if (!real_folder) return FALSE;
    if (DetourTransactionBegin() != NO_ERROR) return FALSE;
    if (DetourUpdateThread(GetCurrentThread()) != NO_ERROR ||
        DetourAttach(&(PVOID&)real_folder, isolated_folder) != NO_ERROR ||
        DetourAttach(&(PVOID&)real_factory, isolated_factory) != NO_ERROR) {
        DetourTransactionAbort();
        return FALSE;
    }
    return DetourTransactionCommit() == NO_ERROR;
}
'@ | Set-Content -Encoding ascii -LiteralPath hook.cpp
@'
#include <windows.h>
#include <detours.h>
#include <cstdio>
#include <string>

int wmain(int argc, wchar_t** argv)
{
    if (argc != 3) return 2;
    STARTUPINFOW si{ sizeof(si) };
    si.dwFlags = STARTF_USESHOWWINDOW;
    si.wShowWindow = SW_HIDE;
    PROCESS_INFORMATION pi{};
    std::wstring line = L"\"" + std::wstring(argv[1]) + L"\" -w new new-tab -p " + argv[2];
    if (!DetourCreateProcessWithDllExW(argv[1], line.data(), nullptr, nullptr,
            FALSE, CREATE_SUSPENDED, nullptr, nullptr, &si, &pi, "csp-isolate.dll", nullptr)) {
        fwprintf(stderr, L"DetourCreateProcessWithDllExW: %lu\n", GetLastError());
        return 3;
    }
    ResumeThread(pi.hThread);
    printf("%lu\n", pi.dwProcessId);
    CloseHandle(pi.hThread);
    CloseHandle(pi.hProcess);
    return 0;
}
'@ | Set-Content -Encoding ascii -LiteralPath launch.cpp
@'
@call "%CSP_WT_VSDEV%" -arch=x64 >nul
@if errorlevel 1 exit /b %errorlevel%
@pushd Detours-4.0.1\src
@nmake /nologo
@if errorlevel 1 exit /b %errorlevel%
@popd
@cl /nologo /std:c++17 /EHsc /I Detours-4.0.1\src /LD hook.cpp /link /OUT:csp-isolate.dll /EXPORT:DetourFinishHelperProcess,@1,NONAME Detours-4.0.1\lib.X64\detours.lib shell32.lib ole32.lib runtimeobject.lib uuid.lib
@if errorlevel 1 exit /b %errorlevel%
@cl /nologo /std:c++17 /EHsc /I Detours-4.0.1\src launch.cpp /link /OUT:launch.exe Detours-4.0.1\lib.X64\detours.lib
'@ | Set-Content -Encoding ascii -LiteralPath build.cmd
    cmd.exe /d /c build.cmd
    if ($LASTEXITCODE -ne 0) { throw "Harness compilation failed: $LASTEXITCODE" }
    $portable = Join-Path $spikeRoot 'terminal-1.24.12741.0'
    Copy-Item -LiteralPath csp-isolate.dll -Destination $portable
    New-Item -ItemType File -Path (Join-Path $portable '.portable') | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $portable 'settings') | Out-Null
    $terminalRoot = Join-Path $spikeRoot 'terminal-1.24.12741.0'
    $terminal = Join-Path $terminalRoot 'WindowsTerminal.exe'
    $settings = Join-Path $terminalRoot 'settings\settings.json'
    $env:CSP_WT_LOCAL = Join-Path $spikeRoot 'known-local'
    $env:CSP_WT_PROGRAM = Join-Path $spikeRoot 'known-program'
    $env:CSP_WT_HOOK_LOG = Join-Path $spikeRoot 'hook.log'
    $fragmentDir = Join-Path $env:CSP_WT_LOCAL 'Microsoft\Windows Terminal\Fragments\CspSmoke'
    $fragment = Join-Path $fragmentDir 'fragment.json'
    $fragmentTarget = Join-Path $spikeRoot 'fragment-directory'
    $defaultTarget = Join-Path $spikeRoot 'defaults-directory'
    $profileTarget = Join-Path $spikeRoot 'profile-directory'
    $env:CSP_WT_MARKER = Join-Path $spikeRoot 'observed.txt'
    $observer = Join-Path $spikeRoot 'observe.cmd'
    New-Item -ItemType Directory -Force -Path $fragmentDir,$env:CSP_WT_PROGRAM,$fragmentTarget,$defaultTarget,$profileTarget | Out-Null
    ('@cd>"{0}"' -f $env:CSP_WT_MARKER) | Set-Content -Encoding ascii -LiteralPath $observer
    $ids = '{0caa0dad-35be-5f56-a8ff-afceeeaa6101}', '{61c54bbd-c2c6-5271-96e7-009a87ff44bf}'

    foreach ($case in 'baseline','unknown','updates','defaults','profile','removed') {
        if ($case -in 'baseline','removed') {
            if (Test-Path -LiteralPath $fragment) { Remove-Item -LiteralPath $fragment }
        } else {
            $fragmentIds = if ($case -eq 'unknown') { '{00000000-0000-4000-8000-000000000001}' } else { $ids }
            @{ profiles = @($fragmentIds | ForEach-Object { @{ updates = $_; startingDirectory = $fragmentTarget } }) } |
                ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 -LiteralPath $fragment
        }
        $defaults = @{}
        if ($case -in 'defaults','profile') { $defaults.startingDirectory = $defaultTarget }
        $entries = @($ids | ForEach-Object {
            $entry = @{ guid = $_; commandline = ('"{0}" /d /s /c ""{1}""' -f $env:ComSpec,$observer); closeOnExit = 'never' }
            if ($case -eq 'profile') { $entry.startingDirectory = $profileTarget }
            $entry
        })
        @{ defaultProfile = $ids[0]; firstWindowPreference = 'defaultProfile'; profiles = @{ defaults = $defaults; list = $entries } } |
            ConvertTo-Json -Depth 6 | Set-Content -Encoding utf8 -LiteralPath $settings
        $expected = switch ($case) {
            'updates' { $fragmentTarget }
            'defaults' { $defaultTarget }
            'profile' { $profileTarget }
            default { $env:USERPROFILE }
        }
        foreach ($id in $ids) {
            if (Test-Path -LiteralPath $env:CSP_WT_MARKER) { Remove-Item -LiteralPath $env:CSP_WT_MARKER }
            $terminalPid = [int](& "$spikeRoot\launch.exe" $terminal $id)
            $child = Get-Process -Id $terminalPid
            try {
                $deadline = (Get-Date).AddSeconds(15)
                while (!(Test-Path -LiteralPath $env:CSP_WT_MARKER) -and !$child.HasExited -and (Get-Date) -lt $deadline) {
                    Start-Sleep -Milliseconds 100
                    $child.Refresh()
                }
                if (!(Test-Path -LiteralPath $env:CSP_WT_MARKER)) {
                    $code = if ($child.HasExited) { '0x{0:X8}' -f $child.ExitCode } else { 'running' }
                    throw "Missing observer for $case $id. Terminal exit $code"
                }
                $actual = (Get-Content -LiteralPath $env:CSP_WT_MARKER -Raw).Trim()
                if ($actual -ne $expected) { throw "$case $id expected $expected, observed $actual" }
                "$case $id $actual"
            } finally {
                if (!$child.HasExited) {
                    if (!$child.WaitForExit(750)) { $child.Kill(); $child.WaitForExit() }
                }
            }
        }
    }
    $redirects = Get-Content -LiteralPath $env:CSP_WT_HOOK_LOG
    if ("CSP_WT_LOCAL -> $env:CSP_WT_LOCAL" -notin $redirects -or
        "CSP_WT_PROGRAM -> $env:CSP_WT_PROGRAM" -notin $redirects) {
        throw 'Expected redirects were not observed.'
    }
    "12 observations passed. Test evidence: $spikeRoot"
} finally {
    Pop-Location
    foreach ($variable in 'CSP_WT_LOCAL','CSP_WT_PROGRAM','CSP_WT_MARKER','CSP_WT_HOOK_LOG','CSP_WT_VSDEV') {
        Remove-Item -LiteralPath "Env:$variable" -ErrorAction SilentlyContinue
    }
}
