$ErrorActionPreference = 'SilentlyContinue'
$procs = Get-CimInstance Win32_Process -Filter "Name='cargo.exe' or Name='rustc.exe'"
"count=$($procs.Count)"
foreach ($p in $procs) {
    $cl = ($p.CommandLine -replace '\s+', ' ')
    $short = $cl
    if ($short.Length -gt 150) { $short = $short.Substring(0, 150) }
    "{0} pid={1} start={2}" -f $p.Name, $p.ProcessId, $p.CreationDate.ToString('HH:mm:ss')
    "    $short"
}
