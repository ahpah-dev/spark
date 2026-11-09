# Builds the finished Spark program and puts it in a folder (and a zip) that you can give to anyone.
#
# Run it from this folder:   powershell -ExecutionPolicy Bypass -File package.ps1
#
# You get:   dist\Spark\Spark.exe       (double-click it!)
#            dist\Spark\games\          (the example games, ready to open)
#            dist\Spark-windows.zip     (the same folder, zipped, to share)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "Building Spark (this takes a few minutes the first time)..."
cargo build --release
if ($LASTEXITCODE -ne 0) { throw "The build failed." }

$out = Join-Path $PSScriptRoot "dist\Spark"
if (Test-Path $out) { Remove-Item $out -Recurse -Force }
New-Item -ItemType Directory -Path $out | Out-Null
Copy-Item "target\release\spark.exe" (Join-Path $out "Spark.exe")

# Put the example games in a "games" folder next to the program. (The program is a window app,
# so we wait for it to finish with -Wait.)
Start-Process -FilePath (Join-Path $out "Spark.exe") -ArgumentList "examples" -WorkingDirectory $out -Wait

@"
SPARK GAME MAKER

Double-click Spark.exe to open the game maker.

 * Your games are saved in the "games" folder next to Spark.exe.
 * Press the green PLAY button to play your game.
 * Press "Examples" to open finished games to play and change.
 * Press "Share" to make a folder with your game that friends can play.
 * Drag a PNG picture onto the window to use it as the picture of the object you are working on.

Keep this folder somewhere you can save files (like your Desktop or Documents),
not inside "Program Files".
"@ | Set-Content -Path (Join-Path $out "READ ME.txt") -Encoding UTF8

$zip = Join-Path $PSScriptRoot "dist\Spark-windows.zip"
if (Test-Path $zip) { Remove-Item $zip -Force }
Compress-Archive -Path $out -DestinationPath $zip

$size = [math]::Round((Get-Item (Join-Path $out "Spark.exe")).Length / 1MB, 1)
Write-Host ""
Write-Host "Done! Spark.exe is $size MB."
Write-Host "Folder: $out"
Write-Host "Zip:    $zip"
