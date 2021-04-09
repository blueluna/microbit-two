use crate::Error;
use core::convert::{From, Into};
use embedded_hal::{
    blocking::{delay::DelayUs, spi::Write},
    digital::v2::OutputPin,
};
pub enum Command {
    DrawLines3bit(u8),
    DrawLines1bit(u8),
    DrawLines4bit(u8),
    NoUpdate,
    Clear,
    BlinkOff,
    BlinkBlack,
    BlinkWhite,
    InvertOn,
    InvertOff,
}

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

pub struct Lpm013m126a<SPI, CS, DISP> {
    spi: SPI,
    cs: CS,
    display: DISP,
    buffer: [u8; 16384],
}

const DISPLAY_WIDTH: usize = 176;
const DISPLAY_HEIGHT: usize = 176;
const LINE_WIDTH_1BIT: usize = DISPLAY_WIDTH / 8;
const LINE_WIDTH_3BIT: usize = (DISPLAY_WIDTH * 3) / 8;
const LINE_WIDTH_4BIT: usize = (DISPLAY_WIDTH * 4) / 8;

impl<SPI, CS, DISP> Lpm013m126a<SPI, CS, DISP>
where
    SPI: Write<u8>,
    CS: OutputPin,
    DISP: OutputPin,
{
    pub fn new(spi: SPI, cs: CS, display: DISP) -> Self {
        Self {
            spi,
            cs,
            display,
            buffer: [0u8; 16384],
        }
    }

    pub fn release(self) -> (SPI, CS, DISP) {
        (self.spi, self.cs, self.display)
    }

    pub fn init(&mut self, delay_source: &mut impl DelayUs<u32>) -> Result<(), Error> {
        self.display.set_low().map_err(|_| Error::DisplayError)?;
        self.cs.set_low().map_err(|_| Error::DisplayError)?;
        self.send_clear()?;
        delay_source.delay_us(1_000);
        self.display.set_high().map_err(|_| Error::DisplayError)?;
        delay_source.delay_us(30);
        delay_source.delay_us(30);
        Ok(())
    }

    fn send(&mut self, data: &[u8]) -> Result<(), Error> {
        defmt::info!("send {=[u8]:x}", data);
        self.cs.set_high().map_err(|_| Error::ChipSelectError)?;
        self.spi.write(data).map_err(|_| Error::BusWriteError)?;
        self.cs.set_low().map_err(|_| Error::ChipSelectError)
    }

    fn send_buffer(&mut self, size: usize) -> Result<(), Error> {
        self.cs.set_high().map_err(|_| Error::ChipSelectError)?;
        self.spi
            .write(&self.buffer[..size])
            .map_err(|_| Error::BusWriteError)?;
        self.cs.set_low().map_err(|_| Error::ChipSelectError)
    }

    fn send_clear(&mut self) -> Result<(), Error> {
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

    pub fn send_colour(&mut self, colour: Colour) -> Result<(), Error> {
        let mut offset = 0;
        let colour_nibble = u8::from(colour) << 1;
        self.buffer[offset..offset + 2]
            .copy_from_slice(u16::from(Command::DrawLines4bit(1)).to_be_bytes().as_ref());
        offset += 2;
        for h in 0..DISPLAY_HEIGHT {
            if h != 0 {
                self.buffer[offset] = 0;
                self.buffer[offset + 1] = (h + 1) as u8;
                offset += 2;
            }
            for w in 0..LINE_WIDTH_4BIT {
                self.buffer[offset + w] = colour_nibble | colour_nibble << 4;
            }
            offset += LINE_WIDTH_4BIT;
        }
        self.buffer[offset] = 0;
        self.buffer[offset + 1] = 0;
        offset += 2;
        self.send_buffer(offset)?;
        Ok(())
    }
}
