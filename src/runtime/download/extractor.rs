//! Generic extractor for runtime archives

use crate::runtime::{
    types::RuntimeError,
};
use anyhow::Result;
use std::path::Path;

/// Generic archive extractor
#[derive(Debug)]
pub struct ArchiveExtractor;

impl ArchiveExtractor {
    /// Create a new archive extractor
    pub fn new() -> Self {
        Self
    }

    /// Extract archive to target directory
    pub fn extract(
        &self,
        archive_path: &Path,
        target_dir: &Path,
    ) -> Result<()> {
        // Select extraction method based on file extension
        let extension = archive_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");

        match extension {
            "zip" => self.extract_zip(archive_path, target_dir)?,
            "gz" => self.extract_tar_gz(archive_path, target_dir)?,
            _ => {
                return Err(RuntimeError::ExtractionFailed(format!(
                    "Unsupported compression format: {}",
                    extension
                ))
                .into());
            }
        }

        Ok(())
    }

    /// Extract ZIP file
    fn extract_zip(
        &self,
        archive_path: &Path,
        target_dir: &Path,
    ) -> Result<()> {
        let file = std::fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| RuntimeError::ExtractionFailed(e.to_string()))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| RuntimeError::ExtractionFailed(e.to_string()))?;

            let outpath = match file.enclosed_name() {
                Some(path) => target_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p)?;
                    }
                }
                let mut outfile = std::fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }

            // Set execution permission (Unix system)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
                }
            }
        }

        Ok(())
    }

    /// Extract tar.gz file
    fn extract_tar_gz(
        &self,
        archive_path: &Path,
        target_dir: &Path,
    ) -> Result<(), RuntimeError> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let file = std::fs::File::open(archive_path)?;
        let gz = GzDecoder::new(file);
        let mut archive = Archive::new(gz);

        archive
            .unpack(target_dir)
            .map_err(|e| RuntimeError::ExtractionFailed(e.to_string()))?;

        Ok(())
    }
}
