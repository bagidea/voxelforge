$ErrorActionPreference = "Continue"
$root = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge"
Set-Location $root
Remove-Item "$root\build_matmaps.done","$root\build_matmaps.fail" -ErrorAction SilentlyContinue
$log = "$root\build_matmaps.log"
"=== START $(Get-Date -Format o) ===" | Out-File -FilePath $log -Encoding utf8
& cargo build --release --bin voxelforge_shot --bin voxelforge -j 2 *>> $log
$code = $LASTEXITCODE
"=== EXIT $code $(Get-Date -Format o) ===" | Out-File -FilePath $log -Append -Encoding utf8
if ($code -eq 0) { "ok" | Out-File "$root\build_matmaps.done" -Encoding utf8 }
else { "exit=$code" | Out-File "$root\build_matmaps.fail" -Encoding utf8 }
