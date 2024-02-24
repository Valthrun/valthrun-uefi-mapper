# Mount-VHD -Path .\UEFI_Boot.vhdx
$SourcePath = "_vm_disks\UEFI_Boot.vhdx"
$MountedPath = "$PSScriptRoot\_vm_disks\mounted\"

cargo b --target x86_64-unknown-uefi
if(-not $?) {
    exit 1;
}

$ErrorActionPreference = 'SilentlyContinue'
Dismount-VHD $SourcePath # Just in case
$ErrorActionPreference = 'Stop'

if (!(Test-Path $MountedPath -PathType Container)) {
    New-Item -ItemType Directory -Force -Path $MountedPath
    if(-not $?) {
        exit 1;
    }
}

Mount-VHD -Path $SourcePath -NoDriveLetter -Passthru | `
    Get-Disk | `
    Get-Partition | `
    where { ($_ | Get-Volume) -ne $Null } | `
    Select-Object -first 1 | `
    Add-PartitionAccessPath -AccessPath $MountedPath

# if(-not $?) {
#     exit 1;
# }

Write-Host "Copy efi file"
cp ".\target\x86_64-unknown-uefi\debug\valthrun-uefi.efi" "$MountedPath\EFI\Boot\Bootx64.efi"
if(-not $?) {
    exit 1;
}

Dismount-VHD $SourcePath
Write-Host "Finished..."