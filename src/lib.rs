pub mod core {
    pub mod error;
    pub mod provider;
}

pub mod backends {
    pub mod filebin;
    pub mod temp_sh;
    pub mod tmpfiles;
    pub mod jumpshare;
    // Future: pub mod s3;
    // Future: pub mod gdrive;
}

#[cfg(feature = "ffi")]
pub mod ffi;

// Re-export common items for easier access
pub use core::provider::FileProvider;
pub use core::provider::UploadedFile;
pub use core::error::ProviderError;
pub use backends::filebin::FilebinProvider;
pub use backends::temp_sh::TempShProvider;
pub use backends::tmpfiles::TmpfilesProvider;
pub use backends::jumpshare::JumpshareProvider;
