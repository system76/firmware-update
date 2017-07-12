#!/bin/bash -e

bash uninstall.sh

echo -e "\e[1mInstalling /boot/efi/system76-fu\e[0m" >&2
sudo mkdir -pv /boot/efi/system76-fu
sudo cp -rv build/boot.efi /boot/efi/system76-fu/boot.efi
sudo cp -rv res /boot/efi/system76-fu/res

DISK="$(findmnt -n /boot/efi -o 'MAJ:MIN' | cut -d ':' -f 1)"
PART="$(findmnt -n /boot/efi -o 'MAJ:MIN' | cut -d ':' -f 2)"
DEV="/dev/$(lsblk -n -o 'KNAME,MAJ:MIN' | grep "${DISK}:0" | cut -d ' ' -f 1)"

echo -e "\e[1mCreating Boot1776\e[0m" >&2
sudo efibootmgr -C -b 1776 -d "${DEV}" -p "${PART}" -l '\system76-fu\boot.efi' -L "System76 Firmware Flasher"

echo -e "\e[1mSetting BootNext\e[0m" >&2
sudo efibootmgr -n 1776

echo -e "\e[1mInstalled system76-fu\e[0m" >&2
