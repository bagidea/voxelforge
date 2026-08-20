# Dump the error blocks out of a locked cargo log (FileShare::ReadWrite).
param(
    [string]$Log = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_poppy_skyab2\after_build.log",
    [int]$Context = 6
)
$fs = New-Object System.IO.FileStream($Log, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read, [System.IO.FileShare]::ReadWrite)
try { $ms = New-Object System.IO.MemoryStream; $fs.CopyTo($ms); $bytes = $ms.ToArray() } finally { $fs.Dispose() }
$txt = if ($bytes.Length -ge 2 -and $bytes[0] -eq 0xFF -and $bytes[1] -eq 0xFE) {
    [System.Text.Encoding]::Unicode.GetString($bytes)
} else { [System.Text.Encoding]::UTF8.GetString($bytes) }
$lines = $txt -split "`r?`n"
Write-Output "LOG=$Log lines=$($lines.Count) errorLines=$(@($lines | Where-Object { $_ -match '^error' }).Count)"
for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '^error') {
        Write-Output "--- @line $($i+1) ---"
        $end = [Math]::Min($i + $Context, $lines.Count - 1)
        for ($j = $i; $j -le $end; $j++) { Write-Output $lines[$j] }
    }
}
