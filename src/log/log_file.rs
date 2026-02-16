use ratatui::style::Color;
#[cfg(not(target_os = "windows"))]
use std::os::fd::{FromRawFd, IntoRawFd, RawFd};
#[cfg(target_os = "windows")]
use std::os::windows::io::{FromRawHandle, IntoRawHandle, RawHandle};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

#[derive(Debug)]
pub struct LogFile {
    pub name: Arc<str>,
    // Use `RawFd`/`RawHandle` to get around self referantial struct
    #[cfg(not(target_os = "windows"))]
    file: RawFd,
    #[cfg(target_os = "windows")]
    file: RawHandle,
    mmap: memmap2::Mmap,
    pub color: Color,
}

impl LogFile {
    /// The color of the log file is calculated from the name of the file.
    /// Files that have the exact same name will have the same color.
    #[must_use] pub fn new(name: Arc<str>) -> Self {
        let file = std::fs::File::open(&*name).expect("Can open file");

        #[cfg(target_os = "windows")]
        let file = file.into_raw_handle();
        #[cfg(not(target_os = "windows"))]
        let file = file.into_raw_fd();

        let mmap = unsafe { memmap2::Mmap::map(file) }.expect("To succeed mmaping");

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

    #[must_use] pub fn contents(&self) -> &[u8] {
        &self.mmap
    }
}

impl Drop for LogFile {
    fn drop(&mut self) {
        // Close the file
        // SAFETY: We are the only owner of the file
        #[cfg(target_os = "windows")]
        let _ = unsafe { std::fs::File::from_raw_handle(self.file) };
        #[cfg(not(target_os = "windows"))]
        let _ = unsafe { std::fs::File::from_raw_fd(self.file) };
    }
}
