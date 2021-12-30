//! # Colours for the JDI LPM013M126A display
//!
//! The display has eigth colours.
//!
//! * Black
//! * Blue
//! * Green
//! * Cyan
//! * Red
//! * Pink
//! * Yellow
//! * White

#[cfg(feature = "graphics")]
use embedded_graphics::pixelcolor::{raw::RawU4, PixelColor};

// impl private::Sealed for RawU3 {}

#[derive(Clone, Copy, PartialEq)]
pub enum Palette8 {
    Black,
    Blue,
    Green,
    Cyan,
    Red,
    Pink,
    Yellow,
    White,
}

impl From<Palette8> for u8 {
    fn from(value: Palette8) -> u8 {
        use Palette8::*;
        match value {
            Black => 0b_0000_0000,
            Blue => 0b_0000_0010,
            Green => 0b_0000_0100,
            Cyan => 0b_0000_0110,
            Red => 0b_0000_1000,
            Pink => 0b_0000_1010,
            Yellow => 0b_0000_1100,
            White => 0b_0000_1110,
        }
    }
}

impl From<u8> for Palette8 {
    fn from(value: u8) -> Palette8 {
        use Palette8::*;
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

#[cfg(feature = "graphics")]
impl PixelColor for Palette8 {
    type Raw = RawU4;
}

impl Palette8 {
    /// Converts two colors into a single byte for the Display
    pub fn colors_byte(a: Palette8, b: Palette8) -> u8 {
        u8::from(a) << 4 | u8::from(b)
    }
}
