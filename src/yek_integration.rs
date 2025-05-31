use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use tokio::process::Command;

/// Yek integration module using embedded binary.
pub struct Yek {
  /// Path to the embedded yek binary.
  yek_binary_path: PathBuf,
}

impl Yek {
  /// Creates a new yek instance.
  /// Extracts the embedded yek binary to a temporary location.
  pub fn new() -> Result<Self> {
    // get the embedded binary path from build script
    let yek_binary_path = Self::extract_embedded_binary()?;

    Ok(Self { yek_binary_path })
  }

  /// Extracts the embedded yek binary to a usable location.
  fn extract_embedded_binary() -> Result<PathBuf> {
    // the build script sets env variable
    let build_binary_path = env!("YEK_BINARY_PATH");

    // create a runtime directory for the binary
    let runtime_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("sif").join("bin");

    std::fs::create_dir_all(&runtime_dir).context("Failed to create runtime directory")?;

    let binary_name = if cfg!(windows) { "yek.exe" } else { "yek" };
    let runtime_binary_path = runtime_dir.join(binary_name);

    // copy the binary if it doesn't exist or is outdated
    if !runtime_binary_path.exists() || Self::is_binary_outdated(&runtime_binary_path, build_binary_path)? {
      std::fs::copy(build_binary_path, &runtime_binary_path).context("Failed to copy yek binary to runtime location")?;

      // make executable on unix systems
      #[cfg(unix)]
      {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&runtime_binary_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&runtime_binary_path, perms)?;
      }
    }

    Ok(runtime_binary_path)
  }

  /// Checks if the runtime binary is outdated compared to the build binary.
  fn is_binary_outdated(runtime_path: &Path, build_path: &str) -> Result<bool> {
    let runtime_metadata = std::fs::metadata(runtime_path)?;
    let build_metadata = std::fs::metadata(build_path)?;

    Ok(runtime_metadata.modified()? < build_metadata.modified()?)
  }

  /// Processes selected files using embedded yek binary.
  /// Returns the serialized content as a string.
  pub async fn process_files(&self, selected_files: &[PathBuf], root_path: &Path) -> Result<String> {
    if selected_files.is_empty() {
      return Err(anyhow::anyhow!("Error: No files selected for processing"));
    }

    // check for extremely large file counts
    // TODO: test yek with large file counts
    if selected_files.len() > 10000 {
      return Err(anyhow::anyhow!("Error: Too many files selected ({}). Yek may fail with large file counts. Please select fewer files.", selected_files.len()));
    }

    // build yek command arguments
    let mut yek_args = Vec::new();

    // add selected files as arguments
    for file_path in selected_files {
      // validate that the file path is within the root directory
      let relative_path = match file_path.strip_prefix(root_path) {
        Ok(rel_path) => rel_path,
        Err(_) => {
          // skip files outside root directory
          eprintln!("Warning: Skipping file outside root directory: {}", file_path.display());
          continue;
        }
      };

      // convert to string
      let path_str = relative_path.to_string_lossy();

      // skip paths that try to escape the root directory
      if path_str.contains("..") {
        eprintln!("Warning: Skipping file with path traversal attempt: {}", path_str);
        continue;
      }

      // skip empty or dangerous paths
      if path_str.is_empty() || path_str.starts_with('-') {
        eprintln!("Warning: Skipping file with invalid path: {}", path_str);
        continue;
      }

      yek_args.push(path_str.to_string());
    }

    // make sure have files to process after validation
    if yek_args.is_empty() {
      return Err(anyhow::anyhow!("Error: No valid files to process after security validation"));
    }

    // warn about large file counts
    if yek_args.len() > 1000 {
      eprintln!("Warning: Large file count ({}), processing may take some time", yek_args.len());
    }

    // execute yek with the selected files
    let output = Command::new(&self.yek_binary_path).args(&yek_args).current_dir(root_path).output().await.context("Failed to execute embedded yek binary")?;

    if output.status.success() {
      let content = String::from_utf8_lossy(&output.stdout);
      Ok(content.to_string())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      Err(anyhow::anyhow!("Error: Yek failed with exit code {}: {}", output.status.code().unwrap_or(-1), stderr))
    }
  }

  /// Copies content to clipboard using platform specific commands.
  pub async fn copy_to_clipboard(&self, content: &str) -> Result<String> {
    use tokio::process::Command;

    // determine the clipboard command based on the platform
    let clipboard_cmd = if cfg!(target_os = "macos") {
      vec!["pbcopy"]
    } else if cfg!(target_os = "linux") {
      // try xclip first, then xsel as fallback
      if Command::new("which").arg("xclip").output().await.is_ok() {
        vec!["xclip", "-selection", "clipboard"]
      } else if Command::new("which").arg("xsel").output().await.is_ok() {
        vec!["xsel", "--clipboard", "--input"]
      } else {
        return Err(anyhow::anyhow!(
          "No clipboard utility found. Please install xclip or xsel:\n\
                     sudo apt-get install xclip  # or\n\
                     sudo apt-get install xsel"
        ));
      }
    } else if cfg!(target_os = "windows") {
      vec!["clip"]
    } else {
      return Err(anyhow::anyhow!("Unsupported platform for clipboard operations"));
    };

    // execute clipboard command
    let mut cmd = Command::new(clipboard_cmd[0]);
    for arg in &clipboard_cmd[1..] {
      cmd.arg(arg);
    }

    let mut child = cmd
      .stdin(std::process::Stdio::piped())
      .stdout(std::process::Stdio::piped())
      .stderr(std::process::Stdio::piped())
      .spawn()
      .context("Failed to spawn clipboard command")?;

    // write content to stdin
    if let Some(stdin) = child.stdin.take() {
      use tokio::io::AsyncWriteExt;
      let mut stdin = stdin;
      stdin.write_all(content.as_bytes()).await.context("Failed to write to clipboard command stdin")?;
      stdin.shutdown().await.context("Failed to close clipboard command stdin")?;
    }

    // wait for command to complete
    let output = child.wait_with_output().await.context("Failed to wait for clipboard command")?;

    if output.status.success() {
      Ok("Content copied to clipboard".to_string())
    } else {
      let stderr = String::from_utf8_lossy(&output.stderr);
      Err(anyhow::anyhow!("Clipboard command failed: {}", stderr))
    }
  }

  /// Processes files and copies to clipboard in one operation.
  /// Main entry point that replaces run_yek function.
  pub async fn run_yek_integrated(&self, selected_files: &[PathBuf], root_path: &Path) -> Result<String> {
    // process files using yek library
    let content = self.process_files(selected_files, root_path).await?;

    // copy to clipboard
    self.copy_to_clipboard(&content).await?;

    Ok(format!("{} files processed and copied to clipboard", selected_files.len()))
  }
}

/// Validates yek options and selected files.
/// Returns a list of warnings if any issues are found.
pub fn validate_yek_options(selected_files: &[PathBuf]) -> Vec<String> {
  let mut warnings = Vec::new();

  if selected_files.is_empty() {
    warnings.push("No files selected".to_string());
  }

  // yek is more permissive than repomix, fewer validations needed
  if selected_files.len() > 1000 {
    warnings.push("Large number of files selected, May take a moment to process".to_string());
  }

  warnings
}
