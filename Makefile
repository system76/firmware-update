TARGET=x86_64-efi-pe

export RUST_TARGET_PATH=$(PWD)/targets
PREFIX=$(PWD)/prefix
LD=$(PREFIX)/bin/$(TARGET)-ld

CARGO=xargo
CARGOFLAGS=--target $(TARGET) --release -- -C soft-float

.phony: all binutils

all: build/boot.iso

build/boot.iso: build/iso/efi/boot/bootx64.efi
	mkisofs -o $@ build/iso

build/iso/efi/boot/bootx64.efi: build/boot.efi
	mkdir -p `dirname $@`
	cp $< $@

build/boot.efi: build/boot.o $(LD)
	$(LD) --oformat pei-x86-64 --subsystem 10 -pie -e _start $< -o $@

build/boot.o: build/boot.a
	rm -rf build/boot
	mkdir build/boot
	cd build/boot && ar x ../boot.a
	ld -r build/boot/*.o -o $@

build/boot.a: src/boot.rs src/* src/*/*
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
