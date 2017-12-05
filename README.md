# watertemp firmware

## Setup [platformio](http://platformio.org)

platformio needs python2 so make sure to create the virtualenv accordingly:
```
# Ubuntu
sudo apt-get install python-virtualenv
virtualenv .env

# ArchLinux
sudo pacman -S python2-virtualenv
virtualenv2 .env
```

Activate the virtualenv and install platformio:
```
. .env/bin/activate
pip install platformio
```

## Build

```
platformio run
```

## Upload

Connect the J-Link

```
./upload.sh
```

## Reset Target

Open an interactive J-Link session and execute a reset followed by a go:
```
LinkExe -device LPC11U24 -speed 4000 -if swd -autoconnect 1
J-Link> r
J-Link> g
```
