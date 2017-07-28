TARGET=x86_64-efi-pe

PREFIX=$(PWD)/prefix
export LD=$(PREFIX)/bin/$(TARGET)-ld
export RUST_TARGET_PATH=$(PWD)/targets
export XARGO_HOME=$(PWD)/build/xargo

CARGO=xargo
CARGOFLAGS=--target $(TARGET) --release -- -C soft-float

all: build/boot.img

clean:
	$(CARGO) clean
	rm -rf build

update:
	git submodule update --init --recursive --remote
	cargo update

qemu: build/boot.img
	qemu-system-x86_64 -enable-kvm -cpu kvm64 -m 1024 -net none -vga cirrus \
		-monitor stdio -bios /usr/share/ovmf/OVMF.fd $<

build/boot.img: build/efi.img
	dd if=/dev/zero of=$@.tmp bs=512 count=100352
	parted $@.tmp -s -a minimal mklabel gpt
	parted $@.tmp -s -a minimal mkpart EFI FAT16 2048s 93716s
	parted $@.tmp -s -a minimal toggle 1 boot
	dd if=$< of=$@.tmp bs=512 count=98304 seek=2048 conv=notrunc
	mv $@.tmp $@

build/efi.img: build/iso/efi/boot/bootx64.efi res/*
	dd if=/dev/zero of=$@.tmp bs=512 count=98304
	mkfs.vfat $@.tmp
	mcopy -i $@.tmp -s build/iso/efi ::
	mmd -i $@.tmp system76-firmware-update
	mcopy -i $@.tmp -s res ::system76-firmware-update
	mv $@.tmp $@

build/boot.iso: build/iso/efi/boot/bootx64.efi
	mkisofs -o $@ build/iso

build/iso/efi/boot/bootx64.efi: build/boot.efi
	mkdir -p `dirname $@`
	cp $< $@

build/boot.efi: build/boot.o $(LD)
	$(LD) \
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
		$< -o $@

build/boot.o: build/boot.a
	rm -rf build/boot
	mkdir build/boot
	cd build/boot && ar x ../boot.a
	ld -r build/boot/*.o -o $@

build/boot.a: Cargo.lock Cargo.toml src/* src/*/*
	mkdir -p build
	$(CARGO) rustc --lib $(CARGOFLAGS) -C lto --emit link=$@

$(LD):
	rm -rf prefix
	mkdir -p prefix/build
	cd prefix/build && \
	../../binutils-gdb/configure --target=x86_64-efi-pe --disable-werror --prefix="$(PREFIX)" && \
	make all-ld -j `nproc` && \
	make install-ld -j `nproc`
