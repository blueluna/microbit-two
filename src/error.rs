pub enum Error {
    /// Unable to write to bus
    BusWriteError,
    /// Unable to write to bus
    ChipSelectError,
    /// Unable to assert display signal
    DisplayError,
    /// Attempted to write to a non-existing pixel outside the display's bounds
    OutOfBoundsError,
}
