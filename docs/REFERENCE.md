# Pinout/GPIO Pins

You can view the pinout of your Raspberry Pi with the `pinout` command. For example, my
pinout command displays:

```
   3V3  (1) (2)  5V    
 GPIO2  (3) (4)  5V    
 GPIO3  (5) (6)  GND   
 GPIO4  (7) (8)  GPIO14
   GND  (9) (10) GPIO15
GPIO17 (11) (12) GPIO18
GPIO27 (13) (14) GND   
GPIO22 (15) (16) GPIO23
   3V3 (17) (18) GPIO24
GPIO10 (19) (20) GND   
 GPIO9 (21) (22) GPIO25
GPIO11 (23) (24) GPIO8 
   GND (25) (26) GPIO7 
 GPIO0 (27) (28) GPIO1 
 GPIO5 (29) (30) GND   
 GPIO6 (31) (32) GPIO12
GPIO13 (33) (34) GND   
GPIO19 (35) (36) GPIO16
GPIO26 (37) (38) GPIO20
   GND (39) (40) GPIO21

vvvvv USB PORTS vvvvvvv
```

I will refer to these pins by their *name*, not by their physical number. For example,
I will refer to _GPIO12_ as *Pin 12*, not as *Pin 32* and to _GPIO4_ as *Pin 4* not as 
*Pin 7*.