param(
  [string]$OutputPath = "src-tauri/icons/icon-source.png",
  [int]$Size = 1024
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing

function New-RoundedRectPath {
  param(
    [float]$X,
    [float]$Y,
    [float]$Width,
    [float]$Height,
    [float]$Radius
  )

  $path = New-Object System.Drawing.Drawing2D.GraphicsPath
  $diameter = $Radius * 2

  $path.AddArc($X, $Y, $diameter, $diameter, 180, 90)
  $path.AddArc($X + $Width - $diameter, $Y, $diameter, $diameter, 270, 90)
  $path.AddArc($X + $Width - $diameter, $Y + $Height - $diameter, $diameter, $diameter, 0, 90)
  $path.AddArc($X, $Y + $Height - $diameter, $diameter, $diameter, 90, 90)
  $path.CloseFigure()

  return $path
}

$resolvedOutput = Join-Path $PWD $OutputPath
$outputDir = Split-Path -Parent $resolvedOutput
if (-not (Test-Path $outputDir)) {
  New-Item -ItemType Directory -Path $outputDir | Out-Null
}

$bitmap = New-Object System.Drawing.Bitmap $Size, $Size
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
$graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
$graphics.Clear([System.Drawing.Color]::Transparent)

$backgroundPath = New-RoundedRectPath -X 96 -Y 96 -Width ($Size - 192) -Height ($Size - 192) -Radius 220
$shadowPath = New-RoundedRectPath -X 118 -Y 126 -Width ($Size - 236) -Height ($Size - 236) -Radius 208

$shadowBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(50, 8, 23, 35))
$graphics.FillPath($shadowBrush, $shadowPath)

$backgroundBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
  (New-Object System.Drawing.Point 96, 96),
  (New-Object System.Drawing.Point ($Size - 96), ($Size - 96)),
  ([System.Drawing.Color]::FromArgb(255, 27, 122, 173)),
  ([System.Drawing.Color]::FromArgb(255, 80, 198, 150))
)
$graphics.FillPath($backgroundBrush, $backgroundPath)

$highlightBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
  (New-Object System.Drawing.Point 160, 120),
  (New-Object System.Drawing.Point 520, 440),
  ([System.Drawing.Color]::FromArgb(70, 255, 255, 255)),
  ([System.Drawing.Color]::FromArgb(0, 255, 255, 255))
)
$graphics.FillEllipse($highlightBrush, 150, 120, 430, 280)

$ringPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(245, 255, 255, 255)), 72
$ringPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$ringPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$ringPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
$graphics.DrawArc($ringPen, 250, 250, 524, 524, 0, 320)

$accentPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 255, 190, 92)), 72
$accentPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$accentPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$accentPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
$graphics.DrawArc($accentPen, 250, 250, 524, 524, 322, 38)

$centerBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 255, 255, 255))
$graphics.FillEllipse($centerBrush, 468, 468, 88, 88)

$handPen = New-Object System.Drawing.Pen ([System.Drawing.Color]::FromArgb(255, 255, 255, 255)), 42
$handPen.StartCap = [System.Drawing.Drawing2D.LineCap]::Round
$handPen.EndCap = [System.Drawing.Drawing2D.LineCap]::Round
$handPen.LineJoin = [System.Drawing.Drawing2D.LineJoin]::Round
$graphics.DrawLine($handPen, 512, 512, 512, 372)
$graphics.DrawLine($handPen, 512, 512, 626, 566)

$pulseBrush = New-Object System.Drawing.SolidBrush ([System.Drawing.Color]::FromArgb(255, 255, 190, 92))
$graphics.FillEllipse($pulseBrush, 688, 236, 82, 82)

$bitmap.Save($resolvedOutput, [System.Drawing.Imaging.ImageFormat]::Png)

$pulseBrush.Dispose()
$handPen.Dispose()
$centerBrush.Dispose()
$accentPen.Dispose()
$ringPen.Dispose()
$highlightBrush.Dispose()
$backgroundBrush.Dispose()
$shadowBrush.Dispose()
$shadowPath.Dispose()
$backgroundPath.Dispose()
$graphics.Dispose()
$bitmap.Dispose()

Write-Output "Generated icon source: $resolvedOutput"
