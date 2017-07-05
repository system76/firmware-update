TARGET=x86_64-efi-pe

PREFIX=$(PWD)/prefix
export LD=$(PREFIX)/bin/$(TARGET)-ld
export RUST_TARGET_PATH=$(PWD)/targets
export XARGO_HOME=$(PWD)/xargo
export XARGO_RUST_SRC=$(PWD)/rust/src

CARGO=xargo
CARGOFLAGS=--target $(TARGET) --release -- -C soft-float

.phony: all binutils qemu

all: build/boot.img

qemu: build/boot.img
	#qemu-system-x86_64 -enable-kvm -cpu kvm64 -m 1024 -net none -vga cirrus \
	qemu-system-x86_64 -cpu qemu64 -m 1024 -net none \
		-monitor stdio -bios /usr/share/ovmf/OVMF.fd $<

build/boot.img: build/efi.img
	dd if=/dev/zero of=$@ bs=512 count=93750
	parted $@ -s -a minimal mklabel gpt
	parted $@ -s -a minimal mkpart EFI FAT16 2048s 93716s
	parted $@ -s -a minimal toggle 1 boot
	dd if=$< of=$@ bs=512 count=91669 seek=2048 conv=notrunc

build/efi.img: build/iso/efi/boot/bootx64.efi
	dd if=/dev/zero of=$@ bs=1024 count=91669
	mformat -i $@ -h 32 -t 32 -n 32 -c 1
	mcopy -i $@ -s build/iso/efi ::

build/boot.iso: build/iso/efi/boot/bootx64.efi
	mkisofs -o $@ build/iso

build/iso/efi/boot/bootx64.efi: build/boot.efi
	mkdir -p `dirname $@`
	cp $< $@

build/boot.efi: build/boot.o $(LD)
	$(LD) --oformat pei-x86-64 --subsystem 10 --pic-executable --entry _start $< -o $@

build/boot.o: build/boot.a
	rm -rf build/boot
	mkdir build/boot
	cd build/boot && ar x ../boot.a
	ld -r build/boot/*.o -o $@

build/boot.a: src/main.rs src/* uefi/src/*
	mkdir -p build
	$(CARGO) rustc --lib $(CARGOFLAGS) -C lto --emit link=$@

clean:
	$(CARGO) clean
	rm -rf build

$(LD):
	rm -rf prefix
	mkdir -p prefix/build
	cd prefix/build && \
	../../binutils-gdb/configure --target=x86_64-efi-pe --disable-werror --prefix="$(PREFIX)" && \
	make all-ld -j `nproc` && \
	make install-ld -j `nproc`
