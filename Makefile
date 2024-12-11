# SPDX-License-Identifier: GPL-3.0-only

TARGET = x86_64-unknown-uefi
BUILD = build/$(TARGET)
QEMU = qemu-system-x86_64
OVMF = /usr/share/OVMF

export BASEDIR ?= system76-firmware-update

all: $(BUILD)/boot.efi

.PHONY: clean
clean:
	cargo clean
	rm -rf build

.PHONY: qemu
qemu: $(BUILD)/boot.img $(OVMF)/OVMF_VARS.fd $(OVMF)/OVMF_CODE.fd
	cp $(OVMF)/OVMF_CODE.fd target/
	cp $(OVMF)/OVMF_VARS.fd target/
	$(QEMU) -enable-kvm -M q35 -m 1024 -vga std \
		-chardev stdio,mux=on,id=debug \
		-device isa-serial,index=2,chardev=debug \
		-device isa-debugcon,iobase=0x402,chardev=debug \
		-drive if=pflash,format=raw,readonly=on,file=target/OVMF_CODE.fd \
		-drive if=pflash,format=raw,readonly=on,file=target/OVMF_VARS.fd \
		-net none \
		$<

$(BUILD)/boot.img: $(BUILD)/efi.img
	dd if=/dev/zero of=$@.tmp bs=512 count=100352
	parted $@.tmp -s -a minimal mklabel gpt
	parted $@.tmp -s -a minimal mkpart EFI FAT16 2048s 93716s
	parted $@.tmp -s -a minimal toggle 1 boot
	dd if=$< of=$@.tmp bs=512 count=98304 seek=2048 conv=notrunc
	mv $@.tmp $@

$(BUILD)/efi.img: $(BUILD)/boot.efi res/*
	dd if=/dev/zero of=$@.tmp bs=512 count=98304
	mkfs.vfat $@.tmp
	mmd -i $@.tmp efi
	mmd -i $@.tmp efi/boot
	mcopy -i $@.tmp $< ::efi/boot/bootx64.efi
	mmd -i $@.tmp $(BASEDIR)
	mcopy -i $@.tmp -s res ::$(BASEDIR)
	if [ -d firmware ]; then mcopy -i $@.tmp -s firmware ::$(BASEDIR); fi
	mv $@.tmp $@

.PHONY: $(BUILD)/boot.efi
$(BUILD)/boot.efi:
	mkdir -p $(@D)
	cargo rustc \
		--target $(TARGET) \
		--release \
		-- \
		--emit link=$@
