# System76 Firmware Update

firmware-update is a UEFI application for applying firmware updates on System76
products.

It is used by [firmware-open](https://github.com/system76/firmware-open) and
the private **firmware** project.

## Flashing firmware

firmware-update expects the firmware images to have specific names:

- `firmware.rom`: SBIOS firmware
- `firmware.cap`: UEFI capsule image
- `ec.rom`: Embedded controller firmware

The mechanism used to apply updates depends on the firmware image:

- coreboot-based SBIOS: firmware-update flashes using [intel-spi](https://github.com/system76/intel-spi)
- System76 EC: firmware-update flashes using [ecflash](https://github.com/system76/ecflash)
- Proprietary: The vendor-provided tools are used
