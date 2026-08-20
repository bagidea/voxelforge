Add-Type -AssemblyName System.Drawing
$p = "E:\Projects\bagidea-ai-agents-office\workspace\projects\Voxelforge\hero-look-final-nohud2.png"
$bmp = [System.Drawing.Bitmap]::FromFile($p)
$w = $bmp.Width; $h = $bmp.Height
$rect = New-Object System.Drawing.Rectangle 0,0,$w,$h
$data = $bmp.LockBits($rect, [System.Drawing.Imaging.ImageLockMode]::ReadOnly, [System.Drawing.Imaging.PixelFormat]::Format24bppRgb)
$stride = $data.Stride
$bytes = New-Object byte[] ($stride * $h)
[System.Runtime.InteropServices.Marshal]::Copy($data.Scan0, $bytes, 0, $bytes.Length)
$bmp.UnlockBits($data)
$bmp.Dispose()

# sRGB <-> linear LUTs
$toLin = New-Object double[] 256
for ($i = 0; $i -lt 256; $i++) {
  $c = $i / 255.0
  if ($c -le 0.04045) { $toLin[$i] = $c / 12.92 } else { $toLin[$i] = [math]::Pow(($c + 0.055) / 1.055, 2.4) }
}
function ToSrgb([double]$l) {
  if ($l -le 0) { return 0.0 }
  if ($l -ge 1) { return 255.0 }
  if ($l -le 0.0031308) { $s = $l * 12.92 } else { $s = 1.055 * [math]::Pow($l, 1.0/2.4) - 0.055 }
  return $s * 255.0
}

$gains = @(1.00, 1.19, 1.30, 1.50, 3.50)
foreach ($g in $gains) {
  $cool = 0; $minDiff = 99999.0; $total = 0
  for ($y = 0; $y -lt $h; $y++) {
    $row = $y * $stride
    for ($x = 0; $x -lt $w; $x++) {
      $i = $row + $x * 3
      $bl = $toLin[$bytes[$i]] * $g
      $b2 = ToSrgb $bl
      $r = [double]$bytes[$i+2]
      $d = $r - $b2
      if ($d -le 0) { $cool++ }
      if ($d -lt $minDiff) { $minDiff = $d }
      $total++
    }
  }
  $pct = [math]::Round(100.0 * $cool / $total, 3)
  Write-Output ("LINEAR B x {0:N2}  ->  cool(B>=R) {1} / {2}  ({3}%)   min(R-B) {4:N1}" -f $g, $cool, $total, $pct, $minDiff)
}
