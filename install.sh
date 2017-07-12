#!/bin/bash -e

if [ "$EUID" != "0" ]
then
    echo "You are not running as root" >&2
    exit 1
fi

bash uninstall.sh

echo -e "\e[1mInstalling /boot/efi/system76-fu\e[0m" >&2
mkdir -pv /boot/efi/system76-fu
cp -rv build/boot.efi /boot/efi/system76-fu/boot.efi
cp -rv res /boot/efi/system76-fu/res

DISK="$(findmnt -n /boot/efi -o 'MAJ:MIN' | cut -d ':' -f 1)"
PART="$(findmnt -n /boot/efi -o 'MAJ:MIN' | cut -d ':' -f 2)"
DEV="/dev/$(lsblk -n -o 'KNAME,MAJ:MIN' | grep "${DISK}:0" | cut -d ' ' -f 1)"

echo -e "\e[1mCreating Boot1776\e[0m" >&2
efibootmgr -C -b 1776 -d "${DEV}" -p "${PART}" -l '\system76-fu\boot.efi' -L "System76 Firmware Update"

echo -e "\e[1mSetting BootNext\e[0m" >&2
efibootmgr -n 1776

echo -e "\e[1mInstalled system76-fu\e[0m" >&2
