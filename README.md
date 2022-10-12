# Description

This program is an **attempt**  to create an I2S interface via a PIO state machine on a Raspberry Pi Pico and outputs a 2kHz sine wave
on it with a sample rate of 44.1KhZ and on two channels (stereo).


Pins are as follows:

  | I2S Signal | GPIO | 
  | ---------- | ---- |
  | DATA       | 15   |
  | BCLK       | 13   |
  | LRCK       | 14   |


BCLK clock frequency should be 44.1 kHz × 16 × 2 = **1.4112 MHz**.

# Problems

- BCLK has a lot of jitter sue to the low value of the clock divisor.
- BCLK frequency is incorrect. 
- Currently using a MAX98357A DAC, but the outputed freqeuncy is about 400Hz too low. 
