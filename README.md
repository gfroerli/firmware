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

```
platformio run --target upload
```

