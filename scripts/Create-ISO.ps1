param(
    [Parameter(Mandatory = $false)]
    [string] $Bootloader,

    [Parameter(Mandatory = $true)]
    [string] $Destination
)

$ErrorActionPreference = "Stop"

$mtools = "$PSScriptRoot\mtools\mtools.exe"
$mkisofs = "$PSScriptRoot\mkisofs\mkisofs.exe"

$Resources = "$PSScriptRoot\..\resources"
$IsoRoot = "$PSScriptRoot\__iso"
$BootImage = "$IsoRoot\EFI\BOOT\efibootfs.img"

Write-Host "Creating ISO"
if (Test-Path $Destination) {
    Remove-Item $Destination -Force
}

# If no specific bootloader has been given, just try to get the latest build
if ([string]::IsNullOrEmpty($Bootloader)) {
    $Bootloader = "release", "debug" | `
        ForEach-Object { Get-Item "target\x86_64-unknown-uefi\$_\valthrun-uefi.efi" } | `
        Sort-Object LastWriteTime -Descending | `
        Select-Object -First 1
    
    Write-Host "Using bootloader $Bootloader"
}

# Create the ISO directory
& {
    $BootImageDir = Split-Path $BootImage
    if (Test-Path $BootImageDir) {
        Remove-Item $BootImageDir -Recurse -Force | Out-Null
    }
    New-Item -Path $BootImageDir -ItemType Directory -Force | Out-Null
}

# Generate the FAT32 content for efi
& {
    $BootImageFile = New-Object System.IO.FileStream $BootImage, Create, ReadWrite
    $BootImageFile.SetLength(20MB) # TODO: Improve file size by calculating the required size
    $BootImageFile.Close()

    & $mtools -c mformat -i "$BootImage" ::
    & $mtools -c mmd -i "$BootImage" "::EFI"
    & $mtools -c mmd -i "$BootImage" "::EFI/BOOT"
    & $mtools -c mcopy -i "$BootImage" $(Resolve-Path "$Resources\PreLoader.efi" -Relative) "::EFI/BOOT/bootx64.efi"
    & $mtools -c mcopy -i "$BootImage" $(Resolve-Path "$Resources\HashTool.efi" -Relative) "::EFI/BOOT/HashTool.efi"
    & $mtools -c mcopy -i "$BootImage" $(Resolve-Path "$Bootloader" -Relative) "::EFI/BOOT/loader.efi"
    # & $mtools -c mdir -i "$BootImage" "::EFI/BOOT"
}

# Create the ISO from root directory
& $mkisofs -o "$Destination" `
    -R -J -d -N `
    -hide-rr-moved `
    -no-emul-boot `
    -eltorito-platform efi `
    -eltorito-boot EFI/BOOT/efibootfs.img `
    -A "VT UEFI Loader" `
    "$IsoRoot"
if (-not $?) { 
    throw "Failed to create ISO ($LastExitCode)"
}
Write-Host "ISO created at $Destination"