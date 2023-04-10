# About the Board

For this project, we'll use a Raspberry Pi 3B+. You can win them in CTF competitions,
or purchase them for reasonable prices online. Other versions of the Raspberry Pi should
work fine for this project, but all documentation will assume you use the same 3B+ I do.

Setting up the board is step 2 after buying your materials.

# Configure Your Board

Plug the SD card into your computer. Download the [Raspi Imager](https://github.com/raspberrypi/rpi-imager) and run it as root (`sudo rpi-imager`).

Click *Choose OS*, then *Raspberry Pi OS (other)* and select *Raspberry Pi OS Lite (32-bit)*. We don't need a desktop environment, so this is plenty.

Next, click *Choose Storage* and select your SD card.

Finally, press Ctrl+Shift+X to open the *Advanced Options* menu. Configure the settings as follows:

- [X] Set Hostname: `grobot.local`
- [X] Enable SSH:
  - [X] Use password authentication
- [X] Set username and password:
  - Username: grobot
  - Password: grobot
- [X] Configure wireless LAN:
  - SSID: YOUR WIFI NAME HERE
  - Password: YOUR WIFI PASSWORD HERE

Click *Save* then *Write* and wait until the write is finished. Unplug the SD card and pop it in your Raspberry Pi.

# Boot Up Your Board

Plug in the power supply that came with your Raspberry Pi to the wall and plug the
Pi into it. It should turn on (you'll see a red light and an intermittent green light
on the board). After about 90 seconds, you should be able to find the IP address of your
Pi by running `ping grobot` if your router provides DNS, or you can log into your router
to find its IP address. You should be able to run:

```sh
$ ssh grobot@IP ADDRESS
```

And log in with the password you set earlier.

# Configure PWM

We will use PWM (Pulse Width Modulation) to control our fans, which we need to set up in
our boot configuration.

## Set up PWM

First, we will enable PWM on Channel 0. We will set up this channel on pin 12,

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
