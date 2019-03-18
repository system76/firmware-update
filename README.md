# System76 Firmware Update

System76 Firmware Update Utility

## Overview

We have put a good amount of effort into designing the most secure firmware delivery system possible. This has involved looking into how updates are handled by other vendors, or often mishandled: https://duo.com/assets/pdf/out-of-box-exploitation_oem-updaters.pdf. In this document, we hope to explain our method of firmware updates to build confidence in System76's ability to securely and reliably update customer machines.

You can review the public site layout that is described in this document here: https://firmware.system76.com/master/. It is self-signed to prevent use by non-technical users.

## Source control
- We have a private firmware repository where we store the firmware sources, when we can, and blobs when we cannot get source
- This repository contains a changelog per model that is verified automatically such that changes must be documented in a user-friendly form
- This repository is built and signed automatically
- Each commit must (in this order, failing moves back to the first stage):
    - Be up to date with master
    - Build successfully (on duplicate build machines)
    - Have a code review by two engineers
    - Test on hardware successfully
- After merges, builds, reviews, and tests, the commit may be merged into master

## Building artifacts
- We will be publish our build method soon as free software, before the first automatic firmware update
- There are duplicate build servers
- Each build server uses ECC memory
- Each build runs in memory, in a transient docker container
- Every build is reproducible, such that artifacts for a git revision will be identical, no matter which machine runs the build
- Each build produces a manifest containing the build revision, artifact names, and artifact SHA384
- This manifest is then signed by a highly secure signing server over a serial connection
- Each manifest across multiple build servers for a specific revision must match, or else the build will fail

## Signing artifacts
- We will publish our signing method soon as free software, before the first automatic firmware update
- Each build server has a signing server, a simple piece of hardware stored inside the case of the build server
- The signing server has a hardware RNG, used to initialize an ED25519 signing key, when enough entropy is available.
- The key is stored in memory, and is never accessible to any parties other than the signing server
- The signing server communicates with fixed length, integrity checked messages over serial
- The signing server has no other interfaces, and stores key data in memory
- Every usage of the signing server is stored to flash memory on the signing server in a blockchain.
- This blockchain is also reproduced independently by the build server, and can be verified manually if necessary.
- The SHA384 of the current blockchain must be agreed upon before the next signing message can be successful

## Publishing artifacts
- We will publish our publishing method soon as free software, before the first automatic firmware update
- There is a publishing server that collects the build artifacts
- The publishing server verifies that all build machines produced identical outputs
- The publishing server validates the build machine blockchains against the signing server public keys
- The publishing server moves signed firmware files to firmware.system76.com
- The files are moved atomically from a temporary directory to firmware.system76.com/BUILD_NUMBER
- The firmware.system76.com/BRANCH directory is updated atomically to point to the firmware.system76.com/BUILD_NUMBER directory

## Hosting artifacts
- We will publish our hosting method soon as free software, before the first automatic firmware update
- The artifacts are hosted in a known location with a static site nginx server
- The only interfaces to this server is over HTTP(S) to view files or SSH with public key authentication
- The only valid SSH key is on the publishing server

## Downloading artifacts
- Our downloading method is already published as free software, as part of the System76 driver.
- The public key of the master signing server is published with our driver
- The driver downloads the newest manifest.sha384.signed, which it verifies against the master key in memory
- This verified data is the SHA384 of the current manifest
- The driver downloads and checks the SHA384 of the manifest file
- The driver then finds the firmware for the current hardware in the manifest file
- The driver downloads and checks the SHA384 of the firmware
- The driver then finds the firmware update frontend in the manifest file
- The driver downloads and checks the SHA384 of the firmware update frontend
- The signed SHA384 of the manifest, manifest, firmware, and firmware update frontend are copied to /boot/efi, and the frontend is set as the next boot order
- The system is rebooted into the firmware update frontend

## Installing artifacts
- The firmware updater frontend is already published as free software, at https://github.com/system76/firmware-update
- The frontend checks the firmware files match the current hardware
- The frontend runs the relevant flashing tools to update the firmware. These tools perform signature checking, but are binary and cannot be trusted.
- The frontend reboots into the host OS
