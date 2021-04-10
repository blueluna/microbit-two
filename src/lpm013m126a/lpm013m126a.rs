//! # JDI LPM013M126A
//! SPI 8 colour memory in pixel (MIP) display
//!
//! ## Connection
//! 
//!
//! 

use crate::{DmaSlice, Error};
use core::convert::From;
use embedded_hal::digital::v2::OutputPin;
use crate::spim::{Instance, Spim};

/// SPI commands for the JDI LPM013M126A
/// The command layout is 
pub enum Command {
    /// Update the specified line in 3-bit colour mode
    DrawLines3bit(u8),
    /// Update the specified line in 1-bit colour mode
    DrawLines1bit(u8),
    /// Update the specified line in 4-bit colour mode
    DrawLines4bit(u8),
    /// No update
    NoUpdate,
    /// Clear screen
    Clear,
    /// Blink off
    BlinkOff,
    /// Blink black
    BlinkBlack,
    /// Blink white
    BlinkWhite,
    /// Inverted colours
    InvertOn,
    /// Normal colours
    InvertOff,
}

/// Number of bits that are line addressing bits
const ADDRESS_BITS: usize = 10;

impl From<Command> for u16 {
    fn from(value: Command) -> u16 {
        use Command::*;
        match value {
            DrawLines3bit(address) => 0b_100000 << ADDRESS_BITS | u16::from(address),
            DrawLines1bit(address) => 0b_100010 << ADDRESS_BITS | u16::from(address),
            DrawLines4bit(address) => 0b_100100 << ADDRESS_BITS | u16::from(address),
            NoUpdate => 0b_000000 << ADDRESS_BITS,
            Clear => 0b_001000 << ADDRESS_BITS,
            BlinkOff => 0b_000000 << ADDRESS_BITS,
            BlinkBlack => 0b_000100 << ADDRESS_BITS,
            BlinkWhite => 0b_000110 << ADDRESS_BITS,
            InvertOn => 0b_000101 << ADDRESS_BITS,
            InvertOff => 0b_000000 << ADDRESS_BITS,
        }
    }
}
#[derive(Copy, Clone)]
pub enum Colour {
    Black,
    Blue,
    Green,
    Cyan,
    Red,
    Pink,
    Yellow,
    White,
}

impl From<Colour> for u8 {
    fn from(value: Colour) -> u8 {
        use Colour::*;
        match value {
            Black => 0x00,
            Blue => 0x01,
            Green => 0x02,
            Cyan => 0x03,
            Red => 0x04,
            Pink => 0x05,
            Yellow => 0x06,
            White => 0x07,
        }
    }
}

impl From<u8> for Colour {
    fn from(value: u8) -> Colour {
        use Colour::*;
        match value {
            1 => Blue,
            2 => Green,
            3 => Cyan,
            4 => Red,
            5 => Pink,
            6 => Yellow,
            7 => White,
            _ => Black,
        }
    }
}

pub struct Lpm013m126a<SPI, DISP> {
    spi: Spim<SPI>,
    display: DISP,
    buffer: [u8; OCTETS_4BIT_LINE_CMD], // buffer holding up to one line of 4-bit pixels
    mode: u8,
    current_line: u8,
    current_colour: u8,
}

const DISPLAY_WIDTH: usize = 176;
const DISPLAY_HEIGHT: usize = 176;
const LINE_WIDTH_3BIT: usize = (DISPLAY_WIDTH * 3) / 8;
const LINE_WIDTH_4BIT: usize = (DISPLAY_WIDTH * 4) / 8;
// octets needed for a 4-bit colour line update
const OCTETS_4BIT_LINE_CMD: usize = LINE_WIDTH_4BIT + 4;
// octets needed for a 4-bit colour full screen update
// const OCTETS_4BIT_SCREEN_CMD: usize = ((LINE_WIDTH_4BIT + 2) * DISPLAY_HEIGHT) + 2;

impl<SPI, DISP> Lpm013m126a<SPI, DISP>
where
    SPI: Instance,
    DISP: OutputPin,
{
    pub fn new(spi: Spim<SPI>, display: DISP) -> Self {
        Self {
            spi,
            display,
            buffer: [0u8; OCTETS_4BIT_LINE_CMD],
            mode: 0,
            current_colour: 0xf,
            current_line: 0,
        }
    }

    pub fn release(self) -> (Spim<SPI>, DISP) {
        (self.spi, self.display)
    }

    pub fn init(&mut self) -> Result<(), Error> {
        self.display.set_low().map_err(|_| Error::DisplayError)?;
        self.display.set_high().map_err(|_| Error::DisplayError)?;
        Ok(())
    }

    pub fn spi_task_event(&mut self) {
        self.spi.clear_write_event();
        if self.mode == 1 {
            self.current_line += 1;
            if self.current_line < DISPLAY_WIDTH as u8 {
                self.send_colour_line(self.current_line, self.current_colour);
            }
            else {
                self.current_line = 1;
            }
        }
    }

    fn send(&mut self, data: &[u8]) -> Result<(), Error> {
        self.spi.start_spi_dma_transfer(DmaSlice::from_slice(data), DmaSlice::null()).map_err(|_| Error::BusWriteError)
    }

    fn send_buffer(&mut self, size: usize) -> Result<(), Error> {
        self.spi.start_spi_dma_transfer(DmaSlice::from_slice(&self.buffer[..size]), DmaSlice::null()).map_err(|_| Error::BusWriteError)
    }

    pub fn send_clear(&mut self) -> Result<(), Error> {
        self.send(u16::from(Command::Clear).to_be_bytes().as_ref())
    }

    pub fn blink_white(&mut self) -> Result<(), Error> {
        self.send(u16::from(Command::BlinkWhite).to_be_bytes().as_ref())
    }

    pub fn send_all_white(&mut self) -> Result<(), Error> {
        let mut offset = 0;
        self.buffer[offset..offset + 2]
            .copy_from_slice(u16::from(Command::DrawLines3bit(1)).to_be_bytes().as_ref());
        offset += 2;
        for h in 0..DISPLAY_HEIGHT {
            if h != 0 {
                self.buffer[offset] = 0;
                self.buffer[offset + 1] = (h + 1) as u8;
                offset += 2;
            }
            for w in 0..LINE_WIDTH_3BIT {
                self.buffer[offset + w] = 0xff;
            }
            offset += LINE_WIDTH_3BIT;
        }
        self.buffer[offset] = 0;
        self.buffer[offset + 1] = 0;
        offset += 2;
        self.send_buffer(offset)
    }

    fn send_colour_line(&mut self, line: u8, colour: u8) -> Result<(), Error> {
        let mut offset = 0;
        self.buffer[offset..offset + 2]
            .copy_from_slice(u16::from(Command::DrawLines4bit(line + 1)).to_be_bytes().as_ref());
        offset += 2;
        for w in 0..LINE_WIDTH_4BIT {
            self.buffer[offset + w] = colour | colour << 4;
        }
        offset += LINE_WIDTH_4BIT;
        self.buffer[offset] = 0;
        self.buffer[offset + 1] = 0;
        offset += 2;
        self.send_buffer(offset)?;
        Ok(())
    }

    pub fn send_colour_lines(&mut self, colour: Colour) -> Result<(), Error> {
        self.current_line = 0;
        self.current_colour = u8::from(colour) << 1;
        self.mode = 1;
        self.send_colour_line(self.current_line, self.current_colour)?;
        Ok(())
    }
}
