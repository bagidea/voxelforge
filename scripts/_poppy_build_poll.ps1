# Poll a detached cargo build loudly: print one status line per round.
# Reads the log with FileShare::ReadWrite (cargo holds it open) and decodes
# UTF-16LE when the BOM says so, so grep-able text comes out either way.
param(
    [int]$Pid_ = 19424,
    [string]$Log = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_skyab2\after_build.log",
    [string]$Exe = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-poppysky\release\voxelforge.exe",
    [int]$Rounds = 3,
    [int]$IntervalSec = 30
)

function Read-LockedText([string]$path) {
    if (-not (Test-Path $path)) { return "" }
    $fs = New-Object System.IO.FileStream($path, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
    try {
        $ms = New-Object System.IO.MemoryStream
        $fs.CopyTo($ms)
        $bytes = $ms.ToArray()
    } finally { $fs.Dispose() }
    if ($bytes.Length -ge 2 -and $bytes[0] -eq 0xFF -and $bytes[1] -eq 0xFE) {
        return [System.Text.Encoding]::Unicode.GetString($bytes)
    }
    return [System.Text.Encoding]::UTF8.GetString($bytes)
}

for ($i = 1; $i -le $Rounds; $i++) {
    $now = (Get-Date).ToString('HH:mm:ss')
    # arm 2 spawns a NEW cargo pid, so liveness is "any cargo on OUR target dir",
    # never the tasklist-name check that lied in the 2026-07-31 scar.
    $ours = @(Get-CimInstance Win32_Process -Filter "Name='cargo.exe'" -ErrorAction SilentlyContinue |
              Where-Object { $_.CommandLine -like '*target-poppysky*' })
    $proc = if ($ours.Count) { $ours[0] } else { $null }
    $alive = if ($proc) { "ALIVE(pid $($proc.ProcessId))" } else { 'GONE' }
    $rustc = @(Get-CimInstance Win32_Process -Filter "Name='rustc.exe'" -ErrorAction SilentlyContinue).Count

    $exeInfo = 'none'
    if (Test-Path $Exe) {
        $e = Get-Item $Exe
        $exeInfo = "$($e.Length) bytes @ $($e.LastWriteTime.ToString('HH:mm:ss'))"
    }

    # the skyab2 cmd builds two arms out of one target dir, so report whichever
    # arm's log is currently growing, and treat build.done as the finish line.
    $out = Split-Path $Log -Parent
    $done = Join-Path $out 'build.done'
    $arm = 'after'
    $beforeLog = Join-Path $out 'before_build.log'
    if (Test-Path $beforeLog) { $Log = $beforeLog; $arm = 'before' }

    $txt = Read-LockedText $Log
    $lines = $txt -split "`r?`n" | Where-Object { $_.Trim() -ne '' }
    $errs = @($lines | Where-Object { $_ -match '^error' }).Count
    $last = if ($lines.Count) { $lines[-1] } else { '(empty)' }
    if ($last.Length -gt 110) { $last = $last.Substring(0, 110) }

    Write-Output "[$now] cargo($Pid_)=$alive rustc=$rustc arm=$arm log=$($txt.Length)ch err=$errs exe=$exeInfo | $last"

    if (Test-Path $done) {
        $code = (Read-LockedText $done).Trim()
        Write-Output "DONE: build.done=$code  errors($arm)=$errs"
        break
    }
    if (-not $proc) { Write-Output "DONE: cargo process gone (no BUILD_EXIT line)  errors=$errs"; break }
    if ($i -lt $Rounds) { Start-Sleep -Seconds $IntervalSec }
}
