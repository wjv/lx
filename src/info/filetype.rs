//! Tests for various types of file (video, image, compressed, etc).
//!
//! Currently this is dependent on the file’s name and extension, because
//! those are the only metadata that we have access to without reading the
//! file’s contents.

use crate::fs::File;
use crate::output::icons::FileIcon;


#[derive(Debug, Default, PartialEq, Eq)]
pub struct FileExtensions;

impl FileExtensions {

    fn is_image(&self, file: &File<'_>) -> bool {
        file.extension_is_one_of( &[
            "png", "jfi", "jfif", "jif", "jpe", "jpeg", "jpg", "gif", "bmp",
            "tiff", "tif", "ppm", "pgm", "pbm", "pnm", "webp", "raw", "arw",
            "svg", "stl", "eps", "dvi", "ps", "cbr", "jpf", "cbz", "xpm",
            "ico", "cr2", "orf", "nef", "heif", "avif", "jxl", "j2k", "jp2",
            "j2c", "jpx",
        ])
    }

    fn is_video(&self, file: &File<'_>) -> bool {
        file.extension_is_one_of( &[
            "avi", "flv", "m2v", "m4v", "mkv", "mov", "mp4", "mpeg",
            "mpg", "ogm", "ogv", "vob", "wmv", "webm", "m2ts", "heic",
        ])
    }

    fn is_music(&self, file: &File<'_>) -> bool {
        file.extension_is_one_of( &[
            "aac", "m4a", "mp3", "ogg", "wma", "mka", "opus",
        ])
    }

    // Lossless music, rather than any other kind of data...
    fn is_lossless(&self, file: &File<'_>) -> bool {
        file.extension_is_one_of( &[
            "alac", "ape", "flac", "wav",
        ])
    }

    // Note: is_crypto, is_document, is_compressed, is_temp,
    // is_compiled, and is_immediate removed — file-type colouring
    // now uses the class/style system.  Only the methods needed
    // for icon assignment (above) are retained.
}


impl FileIcon for FileExtensions {
    fn icon_file(&self, file: &File<'_>) -> Option<char> {
        use crate::output::icons::Icons;

        if self.is_music(file) || self.is_lossless(file) {
            Some(Icons::Audio.value())
        }
        else if self.is_image(file) {
            Some(Icons::Image.value())
        }
        else if self.is_video(file) {
            Some(Icons::Video.value())
        }
        else {
            None
        }
    }
}
