#!/bin/bash

set -euo pipefail

case ${1:-} in
    power-on)
        printf "power on\nexit\n" | JLinkExe
        ;;
    power-off)
        printf "power off\nexit\n" | JLinkExe
        ;;
    power-cycle)
        $0 power-off
        sleep 0.1
        $0 power-on
        ;;
    reset)
        printf "r0\nr1\nexit\n" | JLinkExe
        ;;
    flash)
        objcopy -O ihex "$2" output.hex
        printf "halt\nloadfile output.hex\nr0\nr1\nexit\n" | JLinkExe -speed 4000 -if SWD -device STM32L071KB
        ;;
    *)
        echo "Usage: $0 [power-on,power-off,power-cycle,reset,flash ELF_FILE]"
esac
