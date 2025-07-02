use ratatui::style::Color;
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    os::fd::{FromRawFd, IntoRawFd, RawFd},
    sync::Arc,
};

#[derive(Debug)]
pub struct LogFile {
    pub name: Arc<str>,
    // Use `RawFd` to get around self referantial struct
    file: RawFd,
    mmap: memmap2::Mmap,
    pub color: Color,
}

impl LogFile {
    /// The color of the log file is calculated from the name of the file.
    /// Files that have the exact same name will have the same color.
    pub fn new(name: Arc<str>) -> Self {
        let file = std::fs::File::open(&*name).expect("Can open file");
        let file = file.into_raw_fd();
        let mmap = unsafe { memmap2::Mmap::map(file) }.expect("To succeed mmaping");
        std::str::from_utf8(&mmap).expect("To be valid utf8");

        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        let hash = hasher.finish();

        // The index of the color should be some even number between 2 and 230
        // See the documentation of `Color::Indexed` to see the colors table.
        let color_index = ((hash as u8) % 115 + 1) * 2;
        let color = Color::Indexed(color_index);

        Self {
            name,
            file,
            mmap,
            color,
        }
    }

    pub fn contents(&self) -> &str {
        // SAFETY: The file contents being a valid utf-8 is one of the invariants of this struct.
        unsafe { std::str::from_utf8_unchecked(&self.mmap) }
    }
}

impl Drop for LogFile {
    fn drop(&mut self) {
        // Close the file
        // SAFETY: We are the only owner of the file
        let _ = unsafe { std::fs::File::from_raw_fd(self.file) };
    }
}
