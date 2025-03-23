use rand::Rng as _;
use ratatui::style::Color;
use std::{
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
    pub fn new_with_random_color(name: Arc<str>) -> Self {
        let file = std::fs::File::open(&*name).expect("Can open file");
        let file = file.into_raw_fd();
        let mmap = unsafe { memmap2::Mmap::map(file) }.expect("To succeed mmaping");
        std::str::from_utf8(&mmap).expect("To be valid utf8");
        let color = Color::from_u32(rand::thread_rng().gen_range(255..=0x00FF_FFFF));

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
