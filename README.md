# Grobot

Control your Ikea Rudsta grow cabinet with a Raspberry Pi 3B+! Includes schematics,
diagrams, a BOM, and all the tutorials you need to create a custom grow cabinet for
houseplants, herbs, and propagation.

# Tutorial

If you'd like to replicate this build, you can follow along with the docs!

## Step 0: Reference

Some reference before we start, I recommend glancing at [that document](docs/REFERENCE.md) for some context on the terminology used throughout.

## Step 1: Obtain Materials

Before you start, you'll need some materials and tools, which you can find in the
[materials](docs/MATERIALS.md) doc. Many of the links go to Amazon, but you can buy them
from wherever you prefer to purchase your components. I also recognize that many people
will prefer not to buy Raspberry Pi parts because of [recent events](https://www.buzzfeednews.com/article/chrisstokelwalker/raspberry-pi-hired-ex-cop-mastodon-controversy).
I already had several Pis, so I chose to go ahead an use them (I won't be buying more),
but you can do all of this with an Arduino or Pine64 SOC.  You'll need to figure out the
software and pinouts yourself, but the general idea should transfer.

## Step 2: Board Configuration

Before we install dependencies and set up our software, we need to install our OS and
do some setup of our Raspberry Pi. You can find the directions for doing that in the
[board](docs/BOARD.md) document.

## Step 3: Software Setup

Before we set up our hardware, we need to set up our  Raspberry Pi with the software we
need. It's much easier to set up the software first (up to the point where we
actually run the code) so that we can test the hardware as we work on it. The directions
for the software configuration are [here](docs/SOFTWARE.md). We'll come back to the
software at the end after we have our hardware set up, and during the hardware setup
process we'll create and run small programs to test each part before we screw everything
together.

## Step 4: Hardware Setup

Once we can access our board remotely and we can build the software, we can get started
with hardware setup. This is by far the most complicated part of the electronics and
involves wiring with mains power, so be prepared to spend one or two full days
going through [the hardware document](docs/HARDWARE.md).

## Step 5: Build Cabinet

## Step 6: Add Electronics

## Step 7: Test Electronics

## Step 8: Add Plants