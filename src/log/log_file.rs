use ratatui::style::Color;
#[cfg(not(target_os = "windows"))]
use std::os::fd::{FromRawFd, IntoRawFd, RawFd};
#[cfg(target_os = "windows")]
use std::os::windows::io::{FromRawHandle, IntoRawHandle, RawHandle};
use std::{
    hash::{DefaultHasher, Hash, Hasher},
    io::Read as _,
    path::Path,
    sync::Arc,
};
use tempfile::NamedTempFile;

use crate::log::log_temp_dir;

#[derive(Debug, PartialEq, Eq)]
enum ArchiveType {
    Zip,
    Gzip,
}

#[derive(Debug)]
pub struct LogFile {
    pub name: Arc<Path>,
    // The unzipped temporary file, if the log file is an archive.
    // So it persists for the duration of the program.
    _unzipped_temp: Option<NamedTempFile>,
    // Use `RawFd`/`RawHandle` to get around self referantial struct
    #[cfg(not(target_os = "windows"))]
    file: RawFd,
    #[cfg(target_os = "windows")]
    file: RawHandle,
    mmap: memmap2::Mmap,
    pub color: Color,
}

impl LogFile {
    pub fn new(name: Arc<Path>) -> Self {
        let archive_type = Self::archive_type(&name);
        let temp_file = archive_type.map(|t| Self::unzip(&name, t));
        Self::new_unzipped(name, temp_file)
    }

    /// The color of the log file is calculated from the name of the file.
    /// Files that have the exact same name will have the same color.
    fn new_unzipped(name: Arc<Path>, unzipped_temp: Option<NamedTempFile>) -> Self {
        let file_name = unzipped_temp
            .as_ref()
            .map(|f| f.path().into())
            .unwrap_or_else(|| name.clone());
        let file = std::fs::File::open(&file_name)
            .unwrap_or_else(|_| panic!("Can open file: {:?}", file_name));

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
            _unzipped_temp: unzipped_temp,
            file,
            mmap,
            color,
        }
    }

    pub fn contents(&self) -> &[u8] {
        &self.mmap
    }

    fn archive_type(name: &Path) -> Option<ArchiveType> {
        // Check by extension first — covers zip, gz, gzip, 7z etc.
        if let Some(ext_os) = name.extension() {
            let ext = ext_os.to_string_lossy().to_ascii_lowercase();
            if matches!(ext.as_str(), "zip") {
                return Some(ArchiveType::Zip);
            }
            if matches!(ext.as_str(), "gz" | "gzip") {
                return Some(ArchiveType::Gzip);
            }
        }

        // Fallback: magic bytes at offset 0 — PK\x03\x04 (zip), \x1f\x8b (gz)
        // We check zip and gzip magic here so archives without standard extensions
        // are still detected. Note that gz/7z magic is ambiguous in raw log files,
        // but for archive detection the trade-off is acceptable.
        if let Ok(mut f) = std::fs::File::open(name) {
            let mut buf = [0u8; 4];
            match f.read(&mut buf) {
                Ok(n) if n >= 4 && buf.starts_with(b"PK\x03\x04") => return Some(ArchiveType::Zip),
                Ok(n) if n >= 2 && buf[..2] == [0x1f, 0x8b] => return Some(ArchiveType::Gzip),
                _ => {}
            }
        }
        None
    }

    fn unzip(name: &Path, archive_type: ArchiveType) -> NamedTempFile {
        let tempdir = log_temp_dir::TEMP_DIR.path();
        let mut temp_file =
            NamedTempFile::new_in(tempdir).expect("To succeed creating a temporary file");

        match archive_type {
            ArchiveType::Zip => {
                let mut z = zip::ZipArchive::new(
                    std::fs::File::open(name).expect("To be able to open zip file"),
                )
                .expect("Archive to be valid");

                let mut file = z.by_index(0).expect("Archive to have at least one file");
                std::io::copy(&mut file, temp_file.as_file_mut()).expect("File copy to succeed");
            }
            ArchiveType::Gzip => {
                let mut file = flate2::bufread::GzDecoder::new(std::io::BufReader::new(
                    std::fs::File::open(name).expect("To be able to open gzip file"),
                ));
                std::io::copy(&mut file, temp_file.as_file_mut()).expect("File copy to succeed");
            }
        };

        temp_file
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
