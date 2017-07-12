#!/bin/bash
set -e

if [ -e /boot/efi/system76-fu ]
then
    echo -e "\e[1mRemoving /boot/efi/system76-fu\e[0m" >&2
    sudo rm -rfv /boot/efi/system76-fu
fi

if [ -n "$(sudo efibootmgr | grep '^BootNext')" ]
then
    echo -e "\e[1mUnsetting BootNext\e[0m" >&2
    sudo efibootmgr -N
fi

for label in $(sudo efibootmgr | grep '^Boot[0-9]\{4\}\* System76 Firmware Flasher' | cut -d '*' -f1 | sed 's/Boot//')
do
    echo -e "\e[1mRemoving Boot$label\e[0m" >&2
    sudo efibootmgr -b "$label" -B
done

echo -e "\e[1mUninstalled system76-fu\e[0m" >&2
