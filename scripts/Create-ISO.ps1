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
    $BootFiles = @(
        @{
            "Source" = $(Resolve-Path "$Resources\PreLoader.efi" -Relative)
            "Target" = "::EFI/BOOT/bootx64.efi"
        },
        @{
            "Source" = $(Resolve-Path "$Resources\HashTool.efi" -Relative)
            "Target" = "::EFI/BOOT/HashTool.efi"
        },
        @{
            "Source" = $(Resolve-Path "$Bootloader" -Relative)
            "Target" = "::EFI/BOOT/loader.efi"
        }
    )

    $FileAlignment = 0x200
    foreach ($File in $BootFiles) {
        $File.Size = (Get-Item $File.Source).Length
        if (($File.Size -band ($FileAlignment - 1)) -ne 0) {
            $File.SizePadded = ($File.Size -band (-bnot ($FileAlignment - 1))) + $FileAlignment
        }
        else {
            $File.SizePadded = $File.Size
        }
    }

    $SizeTotal = $($BootFiles | measure-object -property SizePadded -Sum).Sum
    $LastError = $null
    foreach ($Padding in 8, 16, 24, 32, 48, 64, 128) {
        Write-Host "Try padding $Padding"
        try {
            $BootImageFile = New-Object System.IO.FileStream $BootImage, Create, ReadWrite
            $BootImageFile.SetLength($SizeTotal + 1024 * $Padding)
            $BootImageFile.Close()
        
            & $mtools -c mformat -i "$BootImage" :: || $(throw "Failed to format disk")
            & $mtools -c mmd -i "$BootImage" "::EFI" || $(throw "Failed to create folder")
            & $mtools -c mmd -i "$BootImage" "::EFI/BOOT" || $(throw "Failed to create folder")
            foreach ($File in $BootFiles) {
                & $mtools -c mcopy -i "$BootImage" "$($File.Source)" "$($File.Target)" || $(throw "Failed to copy $($File.Source) to $($File.Target)")
            }
            # & $mtools -c mdir -i "$BootImage" "::EFI/BOOT"

            Write-Host "Created FAT system with $Padding additional kb"
            $LastError = $null
            break
        }
        catch {
            $LastError = $_
        }
    }
   
    if ($LastError -ne $null) {
        throw $LastError
    }
}

# Create the ISO from root directory
& $mkisofs -o "$Destination" `
    -R -J -d -N `
    -hide-rr-moved `
    -no-emul-boot `
    -eltorito-platform efi `
    -eltorito-boot EFI/BOOT/efibootfs.img `
    "$IsoRoot"

if (-not $?) { 
    throw "Failed to create ISO ($LastExitCode)"
}
Write-Host "ISO created at $Destination"