#!/bin/bash
# resets the device by pulling the reset line low and then high again
printf "r0\nr1\nexit\n" | JLinkExe
