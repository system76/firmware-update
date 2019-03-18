TARGET?=x86_64-efi-pe
export BASEDIR?=system76-firmware-update

export LD=ld
export RUST_TARGET_PATH=$(CURDIR)/targets
BUILD=build/$(TARGET)

all: $(BUILD)/boot.img

clean:
	cargo clean
	rm -rf build

update:
	git submodule update --init --recursive --remote
	cargo update

qemu: $(BUILD)/boot.img
	kvm -M q35 -m 1024 -net none -vga std -bios /usr/share/OVMF/OVMF_CODE.fd $<

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
	mv $@.tmp $@

$(BUILD)/boot.efi: $(BUILD)/boot.o
	$(LD) \
		-m i386pep \
		--oformat pei-x86-64 \
		--dll \
		--image-base 0 \
		--section-alignment 32 \
		--file-alignment 32 \
		--major-os-version 0 \
		--minor-os-version 0 \
		--major-image-version 0 \
		--minor-image-version 0 \
		--major-subsystem-version 0 \
		--minor-subsystem-version 0 \
		--subsystem 10 \
		--heap 0,0 \
		--stack 0,0 \
		--pic-executable \
		--entry _start \
		--no-insert-timestamp \
		$< -o $@

$(BUILD)/boot.o: $(BUILD)/boot.a
	rm -rf $(BUILD)/boot
	mkdir $(BUILD)/boot
	cd $(BUILD)/boot && ar x ../boot.a
	ld -r $(BUILD)/boot/*.o -o $@

$(BUILD)/boot.a: Cargo.lock Cargo.toml src/* src/*/*
	mkdir -p $(BUILD)
	cargo xrustc \
		--lib \
		--target $(TARGET) \
		--release \
		-- \
		-C soft-float \
		-C lto \
		--emit link=$@
