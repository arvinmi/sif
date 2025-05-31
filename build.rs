use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::Path;

/// Build script that downloads and embeds yek binary.
/// Runs during cargo build and makes sure yek is available.
fn main() -> Result<()> {
  println!("cargo:rerun-if-changed=build.rs");

  let out_dir = env::var("OUT_DIR").context("OUT_DIR not set")?;
  let target = env::var("TARGET").context("TARGET not set")?;

  // find the yek binary name and download URL based on target platform
  let (binary_name, download_url) = get_yek_download_info(&target)?;

  let yek_path = Path::new(&out_dir).join(&binary_name);

  // only download if the binary doesn't exist
  if !yek_path.exists() {
    println!("cargo:warning=Downloading yek binary for {}", target);
    download_yek(&download_url, &yek_path)?;

    // make binary executable on unix systems
    #[cfg(unix)]
    {
      use std::os::unix::fs::PermissionsExt;
      let mut perms = fs::metadata(&yek_path)?.permissions();
      perms.set_mode(0o755);
      fs::set_permissions(&yek_path, perms)?;
    }

    println!("cargo:warning=Yek binary downloaded to {}", yek_path.display());
  }

  // tell cargo where to find the embedded binary
  println!("cargo:rustc-env=YEK_BINARY_PATH={}", yek_path.display());

  Ok(())
}

/// Determines the download URL and binary name for yek based on target platform.
fn get_yek_download_info(target: &str) -> Result<(String, String)> {
  // for reference, yek releases follow this pattern:
  // https://github.com/bodo-run/yek/releases/download/v0.20.0/yek-{platform}.{ext}

  // version pin yek to 0.20.0
  let base_url = "https://github.com/bodo-run/yek/releases/download/v0.20.0";

  let (platform_name, binary_name, extension) = match target {
    // macOS
    "x86_64-apple-darwin" => ("x86_64-apple-darwin", "yek", "tar.gz"),
    "aarch64-apple-darwin" => ("aarch64-apple-darwin", "yek", "tar.gz"),

    // Linux
    "x86_64-unknown-linux-gnu" => ("x86_64-unknown-linux-gnu", "yek", "tar.gz"),
    "aarch64-unknown-linux-gnu" => ("aarch64-unknown-linux-gnu", "yek", "tar.gz"),

    // Windows
    "x86_64-pc-windows-msvc" => ("x86_64-pc-windows-msvc", "yek.exe", "zip"),
    "aarch64-pc-windows-msvc" => ("aarch64-pc-windows-msvc", "yek.exe", "zip"),

    _ => {
      return Err(anyhow::anyhow!(
        "Unsupported target platform: {}. \
                 Yek binaries are available for: \
                 x86_64-apple-darwin, aarch64-apple-darwin, \
                 x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, \
                 x86_64-pc-windows-msvc, aarch64-pc-windows-msvc",
        target
      ));
    }
  };

  let download_url = format!("{}/yek-{}.{}", base_url, platform_name, extension);

  Ok((binary_name.to_string(), download_url))
}

/// Downloads and extracts yek binary from github releases.
fn download_yek(url: &str, target_path: &Path) -> Result<()> {
  // download the archive
  let response = reqwest::blocking::get(url).context("Failed to download yek")?;

  if !response.status().is_success() {
    return Err(anyhow::anyhow!("Failed to download yek: HTTP {}", response.status()));
  }

  let archive_bytes = response.bytes().context("Failed to read yek archive bytes")?;

  // extract the binary based on file extension
  if url.ends_with(".tar.gz") {
    extract_yek_from_tar(&archive_bytes, target_path).context("Failed to extract yek binary from tar.gz")?;
  } else if url.ends_with(".zip") {
    extract_yek_from_zip(&archive_bytes, target_path).context("Failed to extract yek binary from zip")?;
  } else {
    return Err(anyhow::anyhow!("Unsupported archive format: {}", url));
  }

  Ok(())
}

/// Extracts yek binary from tar.gz archive.
fn extract_yek_from_tar(archive_bytes: &[u8], target_path: &Path) -> Result<()> {
  use std::io::Read;

  // decompress the gzip archive
  let tar_data = {
    let mut gz_decoder = flate2::read::GzDecoder::new(archive_bytes);
    let mut tar_data = Vec::new();
    gz_decoder.read_to_end(&mut tar_data).context("Failed to decompress gzip archive")?;
    tar_data
  };

  // extract the tar archive
  let mut archive = tar::Archive::new(&tar_data[..]);

  for entry in archive.entries().context("Failed to read tar entries")? {
    let mut entry = entry.context("Failed to read tar entry")?;
    let path = entry.path().context("Failed to get entry path")?;

    // look for the yek binary (might be in a subdirectory)
    if path.file_name().and_then(|n| n.to_str()) == Some("yek") || path.file_name().and_then(|n| n.to_str()) == Some("yek.exe") {
      // extract the binary
      let mut binary_data = Vec::new();
      entry.read_to_end(&mut binary_data).context("Failed to read binary data")?;

      fs::write(target_path, binary_data).context("Failed to write yek binary")?;

      return Ok(());
    }
  }

  Err(anyhow::anyhow!("Yek binary not found in tar archive"))
}

/// Extracts yek binary from zip archive.
fn extract_yek_from_zip(archive_bytes: &[u8], target_path: &Path) -> Result<()> {
  use std::io::Cursor;

  let cursor = Cursor::new(archive_bytes);
  let mut archive = zip::ZipArchive::new(cursor).context("Failed to open zip")?;

  for i in 0..archive.len() {
    let mut file = archive.by_index(i).context("Failed to read zip entry")?;

    // look for the yek binary
    if file.name().ends_with("yek") || file.name().ends_with("yek.exe") {
      let mut binary_data = Vec::new();
      std::io::copy(&mut file, &mut binary_data).context("Failed to read binary data from zip")?;

      fs::write(target_path, binary_data).context("Failed to write yek binary")?;

      return Ok(());
    }
  }

  Err(anyhow::anyhow!("Yek binary not found in zip archive"))
}
