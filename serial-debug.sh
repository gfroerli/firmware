#!/bin/sh
set -eu
python3 -m serial.tools.miniterm --raw --echo /dev/ttyUSB0 57600
