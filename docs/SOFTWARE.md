# Grobot Software

The software is step 3 of this build, this doc will take you through the software setup
up until the point where we can run the code. Obviously, the code won't actually do 
anything until we have our hardware set up, but we want to get everything configured
before we start gluing things down.

# Dependencies

First, you'll need to install a few dependencies.

## System Dependencies

You won't need to install any system dependencies on your Raspberry Pi to run this code,
but you may want to install `vim` or another text editor to edit it.

## Install Rust

You can install the Rust toolchain for your Raspberry Pi by running this command in your
SSH terminal:

```sh
$ curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

You will see an installation prompt, pick the default option and press `Enter` to
continue.

After the script finishes, it should tell you to run `source $HOME/.cargo/env`. Do that,
then you can check that Rust is installed with:

```sh
$ cargo --help
```

If you don't get an error, congratulations! Rust is installed.

# Setup

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

# Build grobot Program

Run `cargo build --release` in this directory to build the `grobot` program. By default,
it will run with the following settings:

* Fans will run 10 mins at the top of the hour
* Lights will run from 6am-8am and 7pm-11pm local time
* Fans and lights will enable/disable to enforce temperature and humidity thresholds
  by using the lights to burn off some humidity and vice versa

Once you can build the program, you are done with this step! We'll come back to the
software at the end once we are ready to connect everything and start actually using
the cabinet.