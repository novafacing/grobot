# Grobot

Raspberry Pi 3B+ controller for Ikea grow cabinet!

## Setup

### Set up PWM

Enable PWM on channel 1:

```sh
$ sudo bash -c 'echo "dtoverlay=pwm,pin=12,func=4" >> /boot/config.txt'
```

Setup PWM to not require superuser privileges (the part after the command should be
pasted in on stdin):

```sh
$ sudo bash -c 'cat >> /etc/udev/rules.d/99-com.rules'

SUBSYSTEM=="pwm*", PROGRAM="/bin/sh -c '\
    chown -R root:gpio /sys/class/pwm && chmod -R 770 /sys/class/pwm;\
    chown -R root:gpio /sys/devices/platform/soc/*.pwm/pwm/pwmchip* &&\
    chmod -R 770 /sys/devices/platform/soc/*.pwm/pwm/pwmchip*\
'"
```

Reboot:

```sh
$ sudo reboot now
```