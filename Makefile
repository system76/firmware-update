TARGET?=x86_64-unknown-uefi
export BASEDIR?=system76-firmware-update

export LD=ld
export RUST_TARGET_PATH=$(CURDIR)/targets
BUILD=build/$(TARGET)

QEMU?=qemu-system-x86_64
QEMU_FLAGS=\
	-accel kvm \
	-M q35 \
	-m 1024 \
	-net none \
	-vga std \
	-bios /usr/share/OVMF/OVMF_CODE.fd
all: $(BUILD)/boot.img

clean:
	cargo clean
	rm -rf build

update:
	git submodule update --init --recursive --remote
	cargo update

qemu: $(BUILD)/boot.img
	$(QEMU) $(QEMU_FLAGS) $<

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

$(BUILD)/boot.efi: Cargo.lock Cargo.toml src/* src/*/*
	mkdir -p $(BUILD)
	cargo rustc \
		--target $(TARGET) \
		--release \
		-- \
		--emit link=$@
