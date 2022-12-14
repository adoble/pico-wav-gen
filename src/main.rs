//! This program creates an I2S interface via a PIO state machine on a Raspberry Pi Pico and outputs a 2kHz sine wave
//! on it with a sample rate of 44.1KhZ and on two channels (stereo).
//!
//! Pins are as follows:
//! ```text
//!   DATA : GPIO 15
//!   BCLK : GPIO 13
//!   LRCK : GPIO 14
//! ```
//!
//! The sine wave data is stored in `wave.rs`.
//!
//! Using the "offical" example for I2S on the pico at [here](https://github.com/raspberrypi/pico-extras/tree/master/src/rp2_common/pico_audio_i2s)
#![no_std]
#![no_main]

use cortex_m_rt::entry;
use hal::gpio::{FunctionPio0, Pin};

use hal::pac;
use hal::pio::PIOExt;
use hal::Sio;
use panic_halt as _;

use bsp::hal;
use rp_pico as bsp;

mod wave;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();

    let sio = Sio::new(pac.SIO);
    let pins = hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // configure pins for PIO
    let _pin_data: Pin<_, FunctionPio0> = pins.gpio15.into_mode();
    let _pin_bclk: Pin<_, FunctionPio0> = pins.gpio13.into_mode();
    let _pin_lrck: Pin<_, FunctionPio0> = pins.gpio14.into_mode();

    // PIN id for use inside of PIO
    let pin_data_id = 15;
    let pin_bck_id = 13; // BCK and LRCK must be contigouous
    let pin_lrck_id = 14;

    // Define the PIO program for I2S.
    let program_audio_i2s = pio_proc::pio_asm!(
        "
        ;
        ; Copyright (c) 2020 Raspberry Pi (Trading) Ltd.
        ;
        ; SPDX-License-Identifier: BSD-3-Clause
        ;
        
        ; Transmit a mono or stereo I2S audio stream as stereo
        ; This is 16 bits per sample; can be altered by modifying the 'set' params,
        ; or made programmable by replacing 'set x' with 'mov x, y' and using Y as a config register.
        ;
        ; Autopull must be enabled, with threshold set to 32.
        ; Since I2S is MSB-first, shift direction should be to left.
        ; Hence the format of the FIFO word is:
        ;
        ; | 31   :   16 | 15   :    0 |
        ; | sample ws=0 | sample ws=1 |
        ;
        ; Data is output at 1 bit per clock. Use clock divider to adjust frequency.
        ; Fractional divider will probably be needed to get correct bit clock period,
        ; but for common syslck freqs this should still give a constant word select period.
        ;
        ; One output pin is used for the data output.
        ; Two side-set pins are used. Bit 0 is clock, bit 1 is word select.
        
        ; Send 16 bit words to the PIO for mono, 32 bit words for stereo
        
        .side_set 2
        
                            ;        /--- LRCLK
                            ;        |/-- BCLK
        bitloop1:           ;        ||
            out pins, 1       side 0b10
            jmp x-- bitloop1  side 0b11
            out pins, 1       side 0b00
            set x, 14         side 0b01
        
        bitloop0:
            out pins, 1       side 0b00
            jmp x-- bitloop0  side 0b01
            out pins, 1       side 0b10
        public entry_point:
            set x, 14         side 0b11
            "
    );

    // Initialize and start PIO
    let (mut pio, sm0, _, _, _) = pac.PIO0.split(&mut pac.RESETS);

    // The bit clock frequency is the product of the sample rate,  number of bits per channel and  number of channels.
    // So with a sample frequency of 44.1 kHz,  16 bits of precision and two channels (stereo, CD quality)
    // the bit clock frequency is:
    //
    // 44.1 kHz ?? 16 ?? 2 = 1.4112 MHz.
    //
    // This is what we're aiming for with the divisor which assumes  12Mhz clock (as on the pico board)
    let div = 2. as f32;

    // Defines the bit depth
    let _bit_accuracy = 32u32;

    // Install and set up the audio-i2s pio program into the state machine and get a handle to the tx fifo on it.
    let installed = pio.install(&program_audio_i2s.program).unwrap();
    let (mut sm_audio_i2s, _, mut tx_data) = hal::pio::PIOBuilder::from_program(installed)
        .out_pins(pin_data_id, 1)
        .side_set_pin_base(pin_bck_id) // BCK and LRCK are contiguous
        .autopull(true)
        .pull_threshold(32)
        .clock_divisor(div) //?? TODO
        .build(sm0);
    sm_audio_i2s.set_pindirs([
        (pin_data_id, hal::pio::PinDir::Output),
        (pin_bck_id, hal::pio::PinDir::Output),
        (pin_lrck_id, hal::pio::PinDir::Output),
    ]);

    // Start state machine
    sm_audio_i2s.start();

    cortex_m::asm::delay(10);

    let number_samples = wave::WAVE_DATA.len();

    let mut sample_index = 0;

    loop {
        if !tx_data.is_full() {
            // Assemble the left and right channel data
            let mut word = (wave::WAVE_DATA[sample_index] as u32) << 16; // left channel
            word |= wave::WAVE_DATA[sample_index + 1] as u32;

            // Write to the PIO
            tx_data.write(word);

            sample_index = (sample_index + 2) % number_samples;
        }
    }
}
