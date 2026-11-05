# Scarica la release ufficiale di Clink inclusa nell'app e ne verifica l'hash.
$ErrorActionPreference = 'Stop'
$version = '1.9.35'
$file = 'clink.1.9.35.fa48d0.zip'
$sha256 = '5AE6862A703D43FF9250716277FD0811B7B6C344771D17911723E7078D9509F8'
$resources = Join-Path $PSScriptRoot '..\src-tauri\resources'
$target = Join-Path $resources 'clink'
$marker = Join-Path $resources 'clink.sha256'

if ((Test-Path -LiteralPath $target) -and (Test-Path -LiteralPath $marker) -and
    (Get-Content -LiteralPath $marker -Raw).Trim() -eq $sha256) {
    exit 0
}

$temp = Join-Path ([IO.Path]::GetTempPath()) ('csp-clink-' + [guid]::NewGuid())
New-Item -ItemType Directory -Path $temp | Out-Null
try {
    $zip = Join-Path $temp $file
    $ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest -UseBasicParsing -OutFile $zip `
        -Uri "https://github.com/chrisant996/clink/releases/download/v$version/$file"
    $actual = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash
    if ($actual -ne $sha256) {
        throw "Hash di Clink non valido: $actual"
    }
    if (Test-Path -LiteralPath $target) {
        Remove-Item -LiteralPath $target -Recurse -Force
    }
    Expand-Archive -LiteralPath $zip -DestinationPath $target
    Set-Content -LiteralPath $marker -Value $sha256 -Encoding ascii
} finally {
    Remove-Item -LiteralPath $temp -Recurse -Force -ErrorAction SilentlyContinue
}
