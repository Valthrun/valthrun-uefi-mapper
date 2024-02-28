#!/bin/bash

# Usage: ./Create-ISO.sh -v [debug/release] -d [Destination ISO file]

# Check if required tools are installed
command -v mtools >/dev/null 2>&1 || { echo >&2 "mtools is required but it's not installed. Aborting."; exit 1; }
command -v xorriso >/dev/null 2>&1 || { echo >&2 "xorriso is required but it's not installed. Aborting."; exit 1; }
BootloaderVersion="debug"

while getopts v:d: flag
do
    case "${flag}" in
        v) BootloaderVersion=${OPTARG};;
        d) Destination=${OPTARG};;
    esac
done

if [ -z "$Destination" ]; then
    echo "Usage: $0 -v [Version: release/debug] -d [Destination ISO file]"
    exit 1
fi

# Define the path for the bootloader EFI files
if [ "$BootloaderVersion" = "release" ]; then
    Bootloader="../target/x86_64-unknown-uefi/release/valthrun-uefi.efi"
elif [ "$BootloaderVersion" = "debug" ]; then
    Bootloader="../target/x86_64-unknown-uefi/debug/valthrun-uefi.efi"
else
    echo "Invalid value for bootloader version. Please use 'release' or 'debug'."
    exit 1
fi

if [ ! -f "$Bootloader" ]; then
    echo "Bootloader file '$Bootloader' not found. Aborting."
    exit 1
fi

IsoRoot="__iso"
EFI_DIR="$IsoRoot/EFI/BOOT"
EFI_IMG="$EFI_DIR/efibootfs.img"

rm -rf "$IsoRoot"
mkdir -p "$EFI_DIR"

BootloaderSize=$(stat -c%s "$Bootloader")
HashToolSize=$(stat -c%s "../resources/HashTool.efi")
TotalSize=$(($BootloaderSize + $HashToolSize))
Padding=1024
TotalSizeWithPadding=$(($TotalSize + $Padding))

Blocks=$((($TotalSizeWithPadding + 511) / 512))

# Create a FAT image file for EFI boot
dd if=/dev/zero of="$EFI_IMG" bs=512 count=$Blocks
mkfs.vfat "$EFI_IMG"

# Create the required directory structure inside the EFI image
mmd -i "$EFI_IMG" ::EFI
mmd -i "$EFI_IMG" ::EFI/BOOT

# Use mtools to copy the bootloader and other files
MTOOLS_SKIP_CHECK=1 mcopy -i "$EFI_IMG" "$Bootloader" ::/EFI/BOOT/bootx64.efi

MTOOLS_SKIP_CHECK=1 mcopy -i "$EFI_IMG" "../resources/HashTool.efi" ::/EFI/BOOT/HashTool.efi
MTOOLS_SKIP_CHECK=1 mcopy -i "$EFI_IMG" "../resources/PreLoader.efi" ::/EFI/BOOT/loader.efi

# Create the UEFI bootable ISO
xorriso -as mkisofs -o "$Destination" \
    -U -iso-level 3 -full-iso9660-filenames \
    -eltorito-alt-boot \
    -e EFI/BOOT/efibootfs.img \
    -no-emul-boot \
    -isohybrid-gpt-basdat \
    -volid "UEFI_ISO" \
    -no-pad \
    "$IsoRoot"

# Check if ISO creation was successful
if [ $? -ne 0 ]; then
    echo "Failed to create ISO."
    exit 1
else
    echo "ISO created successfully at $Destination"
fi
