# Windows PowerShell 入口：转给 Git Bash 跑同一套 bootstrap.sh。
$ErrorActionPreference = "Stop"
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$bashCandidates = @(
    $env:WINDOWS_GIT_BASH_PATH,
    "C:\Program Files\Git\bin\bash.exe",
    "C:\Program Files (x86)\Git\bin\bash.exe"
)
$bash = $bashCandidates | Where-Object { $_ -and (Test-Path $_) } | Select-Object -First 1
if (-not $bash) {
    Write-Error "未找到 Git Bash。请安装 Git for Windows，或设置 WINDOWS_GIT_BASH_PATH。"
}
& $bash "$scriptDir/bootstrap.sh" @args
exit $LASTEXITCODE
