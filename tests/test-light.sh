#!/bin/bash

echo 26 > /sys/class/gpio/export
echo out > /sys/class/gpio/gpio26/direction
echo 0 > /sys/class/gpio/gpio26/value

sleep 30

echo 1 > /sys/class/gpio/gpio26/value