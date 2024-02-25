$ErrorActionPreference = "Stop"

New-item -Path "driver" -ItemType Directory -Force
if (-Not [string]::IsNullOrEmpty($env:DRIVER_URL)) {
    Write-Host "Downloading target driver from $env:DRIVER_URL"

    Invoke-WebRequest $env:DRIVER_URL -OutFile "driver_uefi.zip" -Headers @{
        "Accept"               = "application/vnd.github+json"
        "Authorization"        = "Bearer $env:DRIVER_URL_AUTHORIZATION"
        "X-GitHub-Api-Version" = "2022-11-28"
    }

    Expand-Archive -Path "driver_uefi.zip" -DestinationPath "driver" -Force
    Remove-Item "driver_uefi.zip"
}
elseif (-not (Test-Path "driver/driver_uefi.dll")) {
    Write-Host "Creating stub driver_uefi.dll"
    Out-File -FilePath "driver/driver_uefi.dll"
}

cargo b -v -r --target x86_64-unknown-uefi
if (-not $?) {
    Write-Host "Failed to build EFI module"
    exit 1
}