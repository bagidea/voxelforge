$ErrorActionPreference = 'SilentlyContinue'
$now = Get-Date
"now: $($now.ToString('yyyy-MM-dd HH:mm:ss'))"

# CPU time of the rose cargo tree
$cargo = Get-Process -Id 4880
if ($cargo) {
    $kids = Get-CimInstance Win32_Process -Filter "ParentProcessId=12780 or ParentProcessId=4880"
    $cpu = ($cargo.CPU + (($kids | ForEach-Object { (Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue).CPU } | Measure-Object -Sum).Sum))
    "rose cargo pid=4880 alive, wall=$((($now - $cargo.StartTime).TotalMinutes).ToString('F1'))min cpu=$([Math]::Round($cpu,1))s childcount=$($kids.Count)"
} else {
    "rose cargo pid=4880 GONE"
}

# newest artifact in target-rose (deps + release root)
$root = 'E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\target-rose'
foreach ($sub in 'release\deps','release') {
    $newest = Get-ChildItem "$root\$sub" -File | Sort-Object LastWriteTime -Descending | Select-Object -First 3
    foreach ($f in $newest) {
        "newest {0}\{1}  {2}  {3}B" -f $sub, $f.Name, $f.LastWriteTime.ToString('HH:mm:ss'), $f.Length
    }
}
# the log tail size
$log = 'E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\_rose_water_build.log'
if (Test-Path $log) { "log size=$((Get-Item $log).Length) mtime=$((Get-Item $log).LastWriteTime.ToString('HH:mm:ss'))" }
