//! # JDI LPM013M126A
//! SPI 8 colour memory in pixel (MIP) display
//!
//! ## Connection
//!
//!
//!

use crate::{
    lpm013m126a::Palette8,
    spim::{Instance, Spim},
    DmaSlice, Error,
};
use core::convert::{From, TryInto};
use embedded_hal::digital::v2::OutputPin;

#[cfg(feature = "graphics")]
use embedded_graphics::{
    draw_target::DrawTarget,
    geometry::{OriginDimensions, Size},
    Pixel,
};

/// SPI commands for the JDI LPM013M126A
/// The command layout is
pub enum Command {
    /// Update the specified line in 3-bit colour mode
    DrawLines3bit,
    /// Update the specified line in 1-bit colour mode
    DrawLines1bit,
    /// Update the specified line in 4-bit colour mode
    DrawLines4bit,
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

impl From<Command> for u8 {
    fn from(value: Command) -> u8 {
        use Command::*;
        match value {
            DrawLines3bit => 0b_1000_0000,
            DrawLines1bit => 0b_1000_1000,
            DrawLines4bit => 0b_1001_0000,
            NoUpdate => 0b_0000_0000,
            Clear => 0b_0010_0000,
            BlinkOff => 0b_0000_0000,
            BlinkBlack => 0b_0001_0000,
            BlinkWhite => 0b_0001_1000,
            InvertOn => 0b_0001_0100,
            InvertOff => 0b_0000_0000,
        }
    }
}

pub struct Lpm013m126a<SPI, DISP> {
    spi: Spim<SPI>,
    display: DISP,
    buffer: [u8; OCTETS_4BIT_LINE_CMD], // buffer holding up to one line of 4-bit pixels
    frame_buffer: [u8; FRAME_BUFFER_SIZE], // buffer holding up to one line of 4-bit pixels
    flags: u32,
    current_line: u8,
}

pub const DISPLAY_WIDTH: u8 = 176;
pub const DISPLAY_HEIGHT: u8 = 176;
const LINE_WIDTH_4BIT: usize = (DISPLAY_WIDTH as usize) >> 1;
// octets needed for a 4-bit colour line update
const OCTETS_4BIT_LINE_CMD: usize = LINE_WIDTH_4BIT + 4;
const FRAME_BUFFER_SIZE: usize = LINE_WIDTH_4BIT * (DISPLAY_HEIGHT as usize);

const FLAGS_NONE: u32 = 0x0000_0000;
const FLAGS_DRAWING: u32 = 0x0000_0001;
const FLAGS_UPDATE: u32 = 0x0000_0002;

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
            frame_buffer: [0u8; FRAME_BUFFER_SIZE],
            flags: FLAGS_NONE,
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
        self.current_line += 1;
        if self.current_line < DISPLAY_WIDTH as u8 {
            let _ = self.send_line(self.current_line);
        } else {
            self.current_line = 0;
            let mut remove_flags = FLAGS_UPDATE;
            if (self.flags & FLAGS_UPDATE) == FLAGS_UPDATE {
                let _ = self.send_line(self.current_line);
            } else {
                remove_flags |= FLAGS_DRAWING;
            }
            self.flags &= !remove_flags;
        }
    }

    fn send(&mut self, data: &[u8]) -> Result<(), Error> {
        self.spi
            .start_spi_dma_transfer(DmaSlice::from_slice(data), DmaSlice::null())
            .map_err(|_| Error::BusWriteError)
    }

    fn send_buffer(&mut self, size: usize) -> Result<(), Error> {
        self.spi
            .start_spi_dma_transfer(DmaSlice::from_slice(&self.buffer[..size]), DmaSlice::null())
            .map_err(|_| Error::BusWriteError)
    }

    fn send_short_command(&mut self, command: Command) -> Result<(), Error> {
        let cmd = [u8::from(command), 0];
        self.send(&cmd)
    }

    pub fn send_clear(&mut self) -> Result<(), Error> {
        self.send_short_command(Command::Clear)
    }

    pub fn blink_white(&mut self) -> Result<(), Error> {
        self.send_short_command(Command::BlinkWhite)
    }

    fn send_line(&mut self, line: u8) -> Result<(), Error> {
        let fb_start = line as usize * LINE_WIDTH_4BIT;
        let fb_end = (line as usize + 1) * LINE_WIDTH_4BIT;
        let slice = &self.frame_buffer[fb_start..fb_end];
        self.buffer[0] = u8::from(Command::DrawLines4bit);
        self.buffer[1] = line;
        self.buffer[2..2 + LINE_WIDTH_4BIT].copy_from_slice(slice);
        self.buffer[2 + LINE_WIDTH_4BIT] = 0;
        self.buffer[2 + LINE_WIDTH_4BIT + 1] = 0;
        self.send_buffer(OCTETS_4BIT_LINE_CMD)?;
        Ok(())
    }

    pub fn set_pixel(&mut self, x: u8, y: u8, colour: Palette8) {
        let c = u8::from(colour);
        let (c, mask) = if x & 1 == 1 {
            (c << 4, 0x0f)
        } else {
            (c, 0xf0)
        };
        let x = (x >> 1) as usize;
        let y = y as usize * LINE_WIDTH_4BIT;
        let i = x + y;
        self.frame_buffer[i] = (self.frame_buffer[i] & mask) | c;
    }

    pub fn update_display(&mut self) -> Result<(), Error> {
        self.flags |= FLAGS_UPDATE;
        if (self.flags & FLAGS_DRAWING) == 0 {
            self.flags |= FLAGS_DRAWING;
            self.send_line(0)
        } else {
            Ok(())
        }
    }
}

#[cfg(feature = "graphics")]
impl<SPI, DISP> DrawTarget for Lpm013m126a<SPI, DISP>
where
    SPI: Instance,
    DISP: OutputPin,
{
    type Color = Palette8;
    // `ExampleDisplay` uses a framebuffer and doesn't need to communicate with the display
    // controller to draw pixel, which means that drawing operations can never fail. To reflect
    // this the type `Infallible` was chosen as the `Error` type.
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        const X_LIMIT: i32 = DISPLAY_WIDTH as i32 - 1;
        const Y_LIMIT: i32 = DISPLAY_HEIGHT as i32 - 1;
        for Pixel(coord, colour) in pixels.into_iter() {
            // Check if the pixel coordinates are out of bounds (negative or greater than
            // (63,63)). `DrawTarget` implementation are required to discard any out of bounds
            // pixels without returning an error or causing a panic.
            if let Ok((x @ 0..=X_LIMIT, y @ 0..=Y_LIMIT)) = coord.try_into() {
                self.set_pixel(x as u8, y as u8, colour);
            }
        }

        Ok(())
    }
}

#[cfg(feature = "graphics")]
impl<SPI, DISP> OriginDimensions for Lpm013m126a<SPI, DISP> {
    fn size(&self) -> Size {
        Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32)
    }
}
