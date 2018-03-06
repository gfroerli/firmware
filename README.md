# watertemp firmware

[![Build status](https://circleci.com/gh/gfroerli/firmware.svg?style=shield&circle-token=:circle-token)](https://circleci.com/gh/gfroerli/firmware)

## Setup [platformio](http://platformio.org)

Install [pipenv](https://github.com/pypa/pipenv) to create a virtualenv with
the correct platformio version:
```
# Ubuntu
sudo apt install python-pip
pip install --user pipenv

# ArchLinux
sudo pacman -S python-pipenv
```

Create and Activate the virtualenv and install platformio:
```
pipenv setup
pipenv shell
```

## Build

```
platformio run
```

## Configure

After the first build, make sure to configure your `src/secrets.h` file with
your LoRaWAN backend secrets. Then, rebuild before uploading to the device.

## Upload

Connect the J-Link

```
./upload.sh
```

## Debugging

Connect to UART1 with 57600 baud:

    miniterm.py </dev/serialport> 57600 --raw

## Reset Target

Open an interactive J-Link session and execute a reset followed by a go:
```
LinkExe -device LPC11U24 -speed 4000 -if swd -autoconnect 1
J-Link> r
J-Link> g
```
