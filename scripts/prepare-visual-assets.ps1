param(
  [string]$SourceDir = "C:\Users\HPK\Downloads\images",
  [string]$ProjectRoot = "."
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Add-Type -AssemblyName System.Drawing
try {
  Add-Type -AssemblyName System.Drawing.Common
}
catch {
}

$helperSource = @"
using System;
using System.Drawing;
using System.Drawing.Drawing2D;
using System.Drawing.Imaging;
using System.IO;
using System.Runtime.InteropServices;

public static class TransparentImageNormalizer
{
    public static Rectangle FindAlphaBounds(Bitmap bitmap, byte alphaThreshold)
    {
        var width = bitmap.Width;
        var height = bitmap.Height;
        var rect = new Rectangle(0, 0, width, height);
        var data = bitmap.LockBits(rect, ImageLockMode.ReadOnly, PixelFormat.Format32bppArgb);

        try
        {
            var stride = Math.Abs(data.Stride);
            var bytes = stride * height;
            var buffer = new byte[bytes];
            Marshal.Copy(data.Scan0, buffer, 0, bytes);

            var minX = width;
            var minY = height;
            var maxX = -1;
            var maxY = -1;

            for (var y = 0; y < height; y++)
            {
                var rowOffset = y * stride;
                for (var x = 0; x < width; x++)
                {
                    var alpha = buffer[rowOffset + (x * 4) + 3];
                    if (alpha < alphaThreshold)
                    {
                        continue;
                    }

                    if (x < minX) minX = x;
                    if (y < minY) minY = y;
                    if (x > maxX) maxX = x;
                    if (y > maxY) maxY = y;
                }
            }

            if (maxX < minX || maxY < minY)
            {
                return Rectangle.Empty;
            }

            return Rectangle.FromLTRB(minX, minY, maxX + 1, maxY + 1);
        }
        finally
        {
            bitmap.UnlockBits(data);
        }
    }

    public static void Normalize(string inputPath, string outputPath, int canvasSize, float occupancy, byte alphaThreshold)
    {
        using var source = new Bitmap(inputPath);
        var bounds = FindAlphaBounds(source, alphaThreshold);
        if (bounds.IsEmpty)
        {
            bounds = new Rectangle(0, 0, source.Width, source.Height);
        }

        using var target = new Bitmap(canvasSize, canvasSize, PixelFormat.Format32bppArgb);
        using var graphics = Graphics.FromImage(target);
        graphics.Clear(Color.Transparent);
        graphics.SmoothingMode = SmoothingMode.HighQuality;
        graphics.InterpolationMode = InterpolationMode.HighQualityBicubic;
        graphics.PixelOffsetMode = PixelOffsetMode.HighQuality;
        graphics.CompositingQuality = CompositingQuality.HighQuality;

        var maxContentSize = canvasSize * occupancy;
        var scale = Math.Min(maxContentSize / bounds.Width, maxContentSize / bounds.Height);
        var drawWidth = bounds.Width * scale;
        var drawHeight = bounds.Height * scale;
        var x = (canvasSize - drawWidth) / 2f;
        var y = (canvasSize - drawHeight) / 2f;

        graphics.DrawImage(
            source,
            new RectangleF(x, y, drawWidth, drawHeight),
            bounds,
            GraphicsUnit.Pixel
        );

        Directory.CreateDirectory(Path.GetDirectoryName(outputPath)!);
        target.Save(outputPath, ImageFormat.Png);
    }
}
"@

$referencedAssemblies = @()
foreach ($assemblyName in @(
  'System.Drawing.Common',
  'System.Drawing',
  'System.Drawing.Primitives',
  'System.Runtime',
  'System.Private.CoreLib',
  'System.Private.Windows.GdiPlus',
  'System.Private.Windows.Core'
)) {
  try {
    $location = [System.Reflection.Assembly]::Load($assemblyName).Location
    if ($location) {
      $referencedAssemblies += $location
    }
  }
  catch {
  }
}

Add-Type -TypeDefinition $helperSource -ReferencedAssemblies $referencedAssemblies

function Resolve-ProjectPath {
  param([string]$RelativePath)

  return [System.IO.Path]::GetFullPath((Join-Path $ProjectRoot $RelativePath))
}

function Ensure-ParentDirectory {
  param([string]$Path)

  $parent = Split-Path -Parent $Path
  if (-not (Test-Path $parent)) {
    New-Item -ItemType Directory -Path $parent | Out-Null
  }
}

function Copy-Asset {
  param(
    [string]$SourceName,
    [string]$TargetRelativePath
  )

  $sourcePath = Join-Path $SourceDir $SourceName
  $targetPath = Resolve-ProjectPath $TargetRelativePath
  Ensure-ParentDirectory $targetPath
  Copy-Item -LiteralPath $sourcePath -Destination $targetPath -Force
  Write-Output "Copied $SourceName -> $TargetRelativePath"
}

function Normalize-Asset {
  param(
    [string]$SourceName,
    [string]$TargetRelativePath,
    [int]$CanvasSize,
    [float]$Occupancy,
    [byte]$AlphaThreshold = 18
  )

  $sourcePath = Join-Path $SourceDir $SourceName
  $targetPath = Resolve-ProjectPath $TargetRelativePath
  [TransparentImageNormalizer]::Normalize($sourcePath, $targetPath, $CanvasSize, $Occupancy, $AlphaThreshold)
  Write-Output "Normalized $SourceName -> $TargetRelativePath"
}

Normalize-Asset "app-icon-main.png" "src/assets/icons/app/app-icon-main.png" 256 0.88
Normalize-Asset "cat-drink.png" "src/assets/illustrations/mascot/cat-drink.png" 640 0.9
Normalize-Asset "cat-rest.png" "src/assets/illustrations/mascot/cat-rest.png" 640 0.9
Normalize-Asset "cat-eye-care.png" "src/assets/illustrations/mascot/cat-eye-care.png" 640 0.9
Normalize-Asset "cat-snooze.png" "src/assets/illustrations/mascot/cat-snooze.png" 640 0.9
Normalize-Asset "empty-reminders.png" "src/assets/illustrations/empty/empty-reminders.png" 720 0.9

Normalize-Asset "reminder-drink-candidate-a.png" "src/assets/icons/reminder/reminder-drink.png" 256 0.72
Normalize-Asset "reminder-rest-candidate-a.png" "src/assets/icons/reminder/reminder-rest.png" 256 0.72
Normalize-Asset "reminder-eye-care-candidate-a.png" "src/assets/icons/reminder/reminder-eye-care.png" 256 0.72

Normalize-Asset "tray-idle.png" "src-tauri/icons/tray-idle.png" 64 0.82
Normalize-Asset "tray-alert.png" "src-tauri/icons/tray-alert.png" 64 0.82
Normalize-Asset "tray-muted.png" "src-tauri/icons/tray-muted.png" 64 0.82
