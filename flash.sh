#!/usr/bin/env bash

set -euo

# To figure out what the device name is:
# plug the device in. Double press the reset button.
# Run `dmesg` and it should list something like:
# [sda] Attached SCSI removable disk
COM_PORT=/dev/ttyACM0
CRATE=$CARGO_PKG_NAME

arm-none-eabi-objcopy -O ihex target/thumbv7em-none-eabi/release/$CRATE target/$CRATE.hex
adafruit-nrfutil dfu genpkg --dev-type 0x0052 --sd-req 0x0123 --application target/$CRATE.hex target/$CRATE.zip
# Use our custom reboot system to boot the controller into serial-only DFU mode.
echo -e "bootloader" > $COM_PORT
# Wait for the reboot.
sleep 1s
adafruit-nrfutil --verbose dfu serial -pkg target/$CRATE.zip -p $COM_PORT -b 115200 --singlebank