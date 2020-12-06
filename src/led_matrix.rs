use crate::hal::{
    gpio::{self, p0, p1},
    prelude::OutputPin,
};

type LED = gpio::Pin<gpio::Output<gpio::PushPull>>;
pub type Image = [[u8; 5]; 5];

const CLEAR_IMAGE: Image = [[0; 5]; 5];

const LEVELS: [u32; 32] = [
    0b_0000_0000_0000_0000_0000_0000_0000_0001,
    0b_0000_0000_0000_0000_0000_0000_0000_0011,
    0b_0000_0000_0000_0000_0000_0000_0000_0111,
    0b_0000_0000_0000_0000_0000_0000_0000_1111,
    0b_0000_0000_0000_0000_0000_0000_0001_1111,
    0b_0000_0000_0000_0000_0000_0000_0011_1111,
    0b_0000_0000_0000_0000_0000_0000_0111_1111,
    0b_0000_0000_0000_0000_0000_0000_1111_1111,
    0b_0000_0000_0000_0000_0000_0001_1111_1111,
    0b_0000_0000_0000_0000_0000_0011_1111_1111,
    0b_0000_0000_0000_0000_0000_0111_1111_1111,
    0b_0000_0000_0000_0000_0000_1111_1111_1111,
    0b_0000_0000_0000_0000_0001_1111_1111_1111,
    0b_0000_0000_0000_0000_0011_1111_1111_1111,
    0b_0000_0000_0000_0000_0111_1111_1111_1111,
    0b_0000_0000_0000_0000_1111_1111_1111_1111,
    0b_0000_0000_0000_0001_1111_1111_1111_1111,
    0b_0000_0000_0000_0011_1111_1111_1111_1111,
    0b_0000_0000_0000_0111_1111_1111_1111_1111,
    0b_0000_0000_0000_1111_1111_1111_1111_1111,
    0b_0000_0000_0001_1111_1111_1111_1111_1111,
    0b_0000_0000_0011_1111_1111_1111_1111_1111,
    0b_0000_0000_0111_1111_1111_1111_1111_1111,
    0b_0000_0000_1111_1111_1111_1111_1111_1111,
    0b_0000_0001_1111_1111_1111_1111_1111_1111,
    0b_0000_0011_1111_1111_1111_1111_1111_1111,
    0b_0000_0111_1111_1111_1111_1111_1111_1111,
    0b_0000_1111_1111_1111_1111_1111_1111_1111,
    0b_0001_1111_1111_1111_1111_1111_1111_1111,
    0b_0011_1111_1111_1111_1111_1111_1111_1111,
    0b_0111_1111_1111_1111_1111_1111_1111_1111,
    0b_1111_1111_1111_1111_1111_1111_1111_1111,
];

pub struct LedMatrix {
    rows: [LED; 5],
    cols: [LED; 5],
    level: u32,
    row: usize,
    buffer: Image,
    next_buffer: Image,
    next_updated: bool,
}

impl LedMatrix {
    /// Initializes all the user LEDs
    pub fn new(
        col1: p0::P0_28<gpio::Output<gpio::PushPull>>,
        col2: p0::P0_11<gpio::Output<gpio::PushPull>>,
        col3: p0::P0_31<gpio::Output<gpio::PushPull>>,
        col4: p1::P1_05<gpio::Output<gpio::PushPull>>,
        col5: p0::P0_30<gpio::Output<gpio::PushPull>>,
        row1: p0::P0_21<gpio::Output<gpio::PushPull>>,
        row2: p0::P0_22<gpio::Output<gpio::PushPull>>,
        row3: p0::P0_15<gpio::Output<gpio::PushPull>>,
        row4: p0::P0_24<gpio::Output<gpio::PushPull>>,
        row5: p0::P0_19<gpio::Output<gpio::PushPull>>,
    ) -> Self {
        let mut led_matrix = Self {
            rows: [
                row1.degrade(),
                row2.degrade(),
                row3.degrade(),
                row4.degrade(),
                row5.degrade(),
            ],
            cols: [
                col1.degrade(),
                col2.degrade(),
                col3.degrade(),
                col4.degrade(),
                col5.degrade(),
            ],
            level: 1,
            row: 0,
            buffer: CLEAR_IMAGE,
            next_buffer: CLEAR_IMAGE,
            next_updated: false,
        };
        // This is needed to reduce flickering on reset
        led_matrix.clear();
        led_matrix
    }

    /// Clear display
    pub fn clear(&mut self) {
        for row in &mut self.rows {
            let _ = row.set_low();
        }
        for col in &mut self.cols {
            let _ = col.set_high();
        }
    }

    /// Display 5x5 display image
    pub fn display(&mut self, image: Image) {
        self.next_buffer.copy_from_slice(&image);
        self.next_updated = true;
    }

    fn swap_buffer(&mut self) {
        // update buffer
        self.buffer = self.next_buffer;
        self.next_updated = false;
    }

    /// Update the display
    pub fn update(&mut self) {
        if self.level == 0x01 {
            self.update_row();
        }
        let row_vals = self.buffer[self.row];
        for (col_pin, col_val) in self.cols.iter_mut().zip(row_vals.iter()) {
            let on = match *col_val {
                0 => false,
                value => {
                    let index = usize::from(value / 8);
                    LEVELS[index] & self.level == self.level
                }
            };
            if on {
                let _ = col_pin.set_low();
            } else {
                let _ = col_pin.set_high();
            }
        }
        self.level = self.level.rotate_left(1);
    }

    /// Prepare to draw the next row
    fn update_row(&mut self) {
        // clear last column
        for col_pin in self.cols.iter_mut() {
            let _ = col_pin.set_high();
        }
        // disable last row
        {
            let row_pin = self.rows.get_mut(self.row).unwrap();
            let _ = row_pin.set_low();
        }
        // update row
        self.row = (self.row + 1) % self.rows.len();
        // update buffer
        if self.row == 0 && self.next_updated {
            self.swap_buffer();
        }
        // new row
        let row_pin = self.rows.get_mut(self.row).unwrap();
        let _ = row_pin.set_high();
    }
}
