./jlink.sh power-on
openocd -f jlink.cfg -f stm32l0.cfg -c 'init; jlink hwstatus; reset; halt'
