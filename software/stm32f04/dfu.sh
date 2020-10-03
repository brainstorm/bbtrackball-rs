#!/usr/bin/bash

cargo embed --release
~/elf2dfuse/elf2dfuse ../../target/thumbv6m-none-eabi/release/stm32f042 ../../target/thumbv6m-none-eabi/release/stm32f042.dfu
dfu-util -a 0 -D ../../target/thumbv6m-none-eabi/release/stm32f042.dfu -R