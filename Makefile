LD := prefix/bin/x86_64-efi-pe-ld

.phony: all binutils

all: build/boot.efi

build/boot.iso: build/iso/efi/boot/bootx64.efi
	mkisofs -o $@ build/iso

build/iso/efi/boot/bootx64.efi: build/boot.efi
	mkdir -p `dirname $@`
	cp $< $@

build/boot.efi: build/boot.o $(LD)
	$(LD) --oformat pei-x86-64 --subsystem 10 -pie -e efi_start $< -o $@

build/boot.o: src/boot.rs src/* src/*/*
	mkdir -p build
	rustc -O --emit=obj --crate-type=lib src/boot.rs --out-dir build/

clean:
	rm -rf build

$(LD):
	rm -rf prefix
	mkdir -p prefix/build
	cd prefix/build && \
	../../binutils-gdb/configure --target=x86_64-efi-pe --disable-werror --prefix="$(PWD)/prefix" && \
	make all-ld -j `nproc` && \
	make install-ld -j `nproc`
