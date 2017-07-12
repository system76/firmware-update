#!/bin/bash
set -e

if [ "$EUID" != "0" ]
then
    echo "You are not running as root" >&2
    exit 1
fi

if [ -e /boot/efi/system76-fu ]
then
    echo -e "\e[1mRemoving /boot/efi/system76-fu\e[0m" >&2
    rm -rfv /boot/efi/system76-fu
else
    echo -e "\e[1mAlready removed /boot/efi/system76-fu\e[0m" >&2
fi

if [ -n "$(efibootmgr | grep '^BootNext: 1776$')" ]
then
    echo -e "\e[1mUnsetting BootNext\e[0m" >&2
    efibootmgr -N
else
    echo -e "\e[1mAlready unset BootNext\e[0m" >&2
fi

if [ -n "$(efibootmgr | grep '^Boot1776\* ')" ]
then
    echo -e "\e[1mRemoving Boot1776\e[0m" >&2
    efibootmgr -B -b 1776
else
    echo -e "\e[1mAlready removed Boot1776\e[0m" >&2
fi

echo -e "\e[1mUninstalled system76-fu\e[0m" >&2
