# kilna-plugin-simple-video installer for Windows:
#   irm https://raw.githubusercontent.com/lacodda/kilna-plugin-simple-video/main/tools/install.ps1 | iex
$ErrorActionPreference = "Stop"

$repo = "lacodda/kilna-plugin-simple-video"

# The tag comes from the /releases/latest redirect rather than the REST API:
# unauthenticated API calls are capped at 60 per hour per IP, and an installer
# that fails because someone else on the same address ran it is no installer.
# $env:KILNA_PLUGIN_SIMPLE_VIDEO_VERSION pins a specific release.
$tag = $env:KILNA_PLUGIN_SIMPLE_VIDEO_VERSION
if (-not $tag) {
    $request = [Net.HttpWebRequest]::Create("https://github.com/$repo/releases/latest")
    $request.AllowAutoRedirect = $false
    $request.UserAgent = "kilna-plugin-simple-video-installer"
    try {
        $response = $request.GetResponse()
        $tag = ($response.Headers["Location"] -split "/")[-1]
        $response.Close()
    } catch {
        throw "Cannot resolve the latest release of ${repo}: $($_.Exception.Message)"
    }
}
if (-not $tag -or $tag -notmatch '^v\d') {
    throw "Cannot resolve the latest release of $repo - set `$env:KILNA_PLUGIN_SIMPLE_VIDEO_VERSION to a tag like v0.1.0"
}

# kilna looks for plugins in the plugins folder beside its workspace.
$dir = if ($env:KILNA_PLUGIN_SIMPLE_VIDEO_DIR) { $env:KILNA_PLUGIN_SIMPLE_VIDEO_DIR } else { Join-Path $env:APPDATA "com.lacodda.kilna\plugins" }
$name = "kilna-plugin-simple-video-$tag-x86_64-pc-windows-msvc"
$url = "https://github.com/$repo/releases/download/$tag/$name.zip"
$tmp = Join-Path ([IO.Path]::GetTempPath()) "kilna-plugin-simple-video-install-$([guid]::NewGuid())"
New-Item -ItemType Directory -Force $tmp | Out-Null

try {
    Write-Host "Downloading $url"
    Invoke-WebRequest $url -OutFile (Join-Path $tmp "plugin.zip")
    Expand-Archive (Join-Path $tmp "plugin.zip") -DestinationPath $tmp -Force
    $exe = Get-ChildItem $tmp -Recurse -Filter "kilna-plugin-simple-video.exe" | Select-Object -First 1
    if (-not $exe) { throw "The release archive holds no kilna-plugin-simple-video.exe" }
    New-Item -ItemType Directory -Force $dir | Out-Null
    Copy-Item $exe.FullName $dir -Force
} finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
Write-Host "Installed kilna-plugin-simple-video $tag to $dir\kilna-plugin-simple-video.exe"

if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Host "Note: ffmpeg is not on your PATH; the plugin needs it (winget install Gyan.FFmpeg)."
}
Write-Host "In kilna, 'Render the video' now stands on a work; its settings are in Settings, Plugins."
