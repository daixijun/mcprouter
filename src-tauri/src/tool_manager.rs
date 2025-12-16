use crate::error::{McpError, Result};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::time::Duration;
use tokio::fs;
use tokio::process::Command as TokioCommand;
use tokio::sync::RwLock;

/// Simple tool manager for Bun, UV, and UVX
#[derive(Debug)]
pub struct ToolManager {
    bin_dir: PathBuf,
    download_client: reqwest::Client,
    tools: RwLock<Vec<crate::types::ToolInfo>>,
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolManager {
    /// Create a new ToolManager instance
    pub fn new() -> Self {
        let bin_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".mcprouter")
            .join("bin");

        Self {
            bin_dir,
            download_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap(),
            tools: RwLock::new(Vec::new()),
        }
    }

    /// Get the bin directory path
    pub fn bin_dir(&self) -> &Path {
        &self.bin_dir
    }

    /// Initialize tools directory
    pub async fn initialize(&self) -> Result<()> {
        fs::create_dir_all(&self.bin_dir).await?;
        Ok(())
    }

    /// Get tool path
    fn get_tool_path(&self, tool_name: &str) -> PathBuf {
        self.bin_dir.join(tool_name)
    }

    /// Check if tool exists
    async fn tool_exists(&self, tool_name: &str) -> bool {
        let tool_path = self.get_tool_path(tool_name);
        tokio::fs::metadata(&tool_path).await.is_ok()
    }

    
    /// Get tool version by running it
    fn get_tool_version(&self, tool_name: &str) -> Option<String> {
        let tool_path = self.get_tool_path(tool_name);

        let output = match StdCommand::new(&tool_path).arg("--version").output() {
            Ok(output) => output,
            Err(_) => return None,
        };

        if output.status.success() {
            String::from_utf8_lossy(&output.stdout)
                .split_whitespace()
                .last()
                .map(|s| s.to_string())
        } else {
            None
        }
    }

    /// Download and install Bun
    async fn install_bun(&self) -> Result<()> {
        if self.tool_exists("bun").await {
            tracing::info!("Bun already exists");
            return Ok(());
        }

        // Determine platform and download URL
        let (os, arch) = self.detect_platform();
        let url = match (os.as_str(), arch.as_str()) {
            ("darwin", "x86_64") => {
                "https://github.com/oven-sh/bun/releases/latest/download/bun-darwin-x64.zip"
            }
            ("darwin", "aarch64") => {
                "https://github.com/oven-sh/bun/releases/latest/download/bun-darwin-aarch64.zip"
            }
            ("linux", "x86_64") => {
                "https://github.com/oven-sh/bun/releases/latest/download/bun-linux-x64.zip"
            }
            ("linux", "aarch64") => {
                "https://github.com/oven-sh/bun/releases/latest/download/bun-linux-aarch64.zip"
            }
            ("windows", "x86_64") => {
                "https://github.com/oven-sh/bun/releases/latest/download/bun-windows-x64.zip"
            }
            _ => return Err(McpError::UnsupportedPlatform(format!("{}-{}", os, arch))),
        };

        // Download and extract
        self.download_and_extract_zip(url, "bun").await
    }

    /// Download and install UV (includes UVX)
    async fn install_uv_tools(&self) -> Result<()> {
        if self.tool_exists("uv").await {
            tracing::info!("UV already exists (includes UVX)");
            return Ok(());
        }

        // Determine platform and download URL
        let (os, arch) = self.detect_platform();
        let url = match (os.as_str(), arch.as_str()) {
            ("darwin", "x86_64") => "https://github.com/astral-sh/uv/releases/latest/download/uv-x86_64-apple-darwin.tar.gz",
            ("darwin", "aarch64") => "https://github.com/astral-sh/uv/releases/latest/download/uv-aarch64-apple-darwin.tar.gz",
            ("linux", "x86_64") => "https://github.com/astral-sh/uv/releases/latest/download/uv-x86_64-unknown-linux-gnu.tar.gz",
            ("linux", "aarch64") => "https://github.com/astral-sh/uv/releases/latest/download/uv-aarch64-unknown-linux-gnu.tar.gz",
            ("windows", "x86_64") => "https://github.com/astral-sh/uv/releases/latest/download/uv-x86_64-pc-windows-msvc.zip",
            _ => return Err(McpError::UnsupportedPlatform(format!("{}-{}", os, arch))),
        };

        // Download and extract UV
        if url.ends_with(".zip") {
            self.download_and_extract_zip(url, "uv").await?;
        } else {
            self.download_and_extract_tar(url).await?;
        }

        tracing::info!("UV installed successfully (includes UVX command)");
        Ok(())
    }

    /// Download and extract ZIP archive
    async fn download_and_extract_zip(&self, url: &str, tool_name: &str) -> Result<()> {
        // Download
        let response = self.download_client.get(url).send().await.map_err(|e| {
            McpError::DownloadError(format!("Failed to download {}: {}", tool_name, e))
        })?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| McpError::DownloadError(format!("Failed to read response: {}", e)))?;

        // Extract in a separate function to avoid Send issues
        self.extract_zip_from_bytes(&bytes, tool_name).await
    }

    /// Extract ZIP from bytes (separated to avoid Send issues)
    async fn extract_zip_from_bytes(&self, bytes: &[u8], tool_name: &str) -> Result<()> {
        use zip::ZipArchive;

        // Extract
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| McpError::DownloadError(format!("Failed to open zip: {}", e)))?;

        // Find the executable file
        let mut file_data = None;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| {
                McpError::DownloadError(format!("Failed to access zip entry: {}", e))
            })?;

            if file.name().ends_with("bun")
                || file.name().ends_with("uv")
                || file.name().ends_with("uv.exe")
            {
                let mut buffer = Vec::new();
                std::io::copy(&mut file, &mut buffer).map_err(|e| {
                    McpError::DownloadError(format!("Failed to read file from zip: {}", e))
                })?;
                file_data = Some(buffer);
                break;
            }
        }

        // Write the file asynchronously
        if let Some(data) = file_data {
            let out_path = self.get_tool_path(tool_name);
            tokio::fs::write(&out_path, data).await?;

            // Set executable permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = tokio::fs::metadata(&out_path).await?.permissions();
                perms.set_mode(0o755);
                tokio::fs::set_permissions(&out_path, perms).await?;
            }
        } else {
            return Err(McpError::DownloadError(format!(
                "Executable not found in archive for {}",
                tool_name
            )));
        }

        Ok(())
    }

    /// Download and extract tar.gz archive
    async fn download_and_extract_tar(&self, url: &str) -> Result<()> {
        // Download
        let response = self.download_client.get(url).send().await.map_err(|e| {
            McpError::DownloadError(format!("Failed to download UV tar.gz: {}", e))
        })?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| McpError::DownloadError(format!("Failed to read response: {}", e)))?;

        // Extract in a separate function to avoid Send issues
        self.extract_tar_from_bytes(&bytes).await
    }

    /// Extract tar.gz from bytes (separated to avoid Send issues)
    async fn extract_tar_from_bytes(&self, bytes: &[u8]) -> Result<()> {
        use flate2::read::GzDecoder;
        use std::io::Read;
        use tar::Archive;

        // Extract both UV and UVX executables to memory
        let cursor = std::io::Cursor::new(bytes);
        let decoder = GzDecoder::new(cursor);
        let mut archive = Archive::new(decoder);

        let mut uv_data = None;
        let mut uvx_data = None;

        for entry in archive
            .entries()
            .map_err(|e| McpError::DownloadError(format!("Failed to read tar entries: {}", e)))?
        {
            let mut entry = entry.map_err(|e| {
                McpError::DownloadError(format!("Failed to access tar entry: {}", e))
            })?;

            let path = entry
                .path()
                .map_err(|e| McpError::DownloadError(format!("Failed to get entry path: {}", e)))?;

            let file_name = path.file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("");

            // Look for both uv and uvx executables
            if file_name == "uv" {
                let mut buffer = Vec::new();
                entry.read_to_end(&mut buffer).map_err(|e| {
                    McpError::DownloadError(format!("Failed to read uv tar entry: {}", e))
                })?;
                uv_data = Some(buffer);
            } else if file_name == "uvx" {
                let mut buffer = Vec::new();
                entry.read_to_end(&mut buffer).map_err(|e| {
                    McpError::DownloadError(format!("Failed to read uvx tar entry: {}", e))
                })?;
                uvx_data = Some(buffer);
            }
        }

        // Write UV executable
        if let Some(data) = uv_data {
            let out_path = self.get_tool_path("uv");
            tokio::fs::write(&out_path, data).await?;

            // Set executable permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = tokio::fs::metadata(&out_path).await?.permissions();
                perms.set_mode(0o755);
                tokio::fs::set_permissions(&out_path, perms).await?;
            }
            tracing::info!("UV executable extracted to {}", out_path.display());
        } else {
            return Err(McpError::DownloadError(
                "UV executable not found in archive".to_string(),
            ));
        }

        // Write UVX executable
        if let Some(data) = uvx_data {
            let out_path = self.get_tool_path("uvx");
            tokio::fs::write(&out_path, data).await?;

            // Set executable permissions
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = tokio::fs::metadata(&out_path).await?.permissions();
                perms.set_mode(0o755);
                tokio::fs::set_permissions(&out_path, perms).await?;
            }
            tracing::info!("UVX executable extracted to {}", out_path.display());
        } else {
            // UVX might not be in all archives, so log a warning instead of error
            tracing::warn!("UVX executable not found in archive (may not be included in this version)");
        }

        Ok(())
    }

    /// Detect current platform
    fn detect_platform(&self) -> (String, String) {
        let os = std::env::consts::OS.to_string();
        let raw_arch = std::env::consts::ARCH;

        // Map Rust OS constant to download URL OS name
        let os_name = match os.as_str() {
            "macos" => "darwin",  // macOS uses "darwin" in download URLs
            "windows" => "windows",
            "linux" => "linux",
            _ => {
                tracing::warn!("Unknown OS: {}, using as-is", os);
                &os
            }
        };

        let arch = match raw_arch.to_lowercase().as_str() {
            "x86_64" => "x86_64".to_string(),
            "aarch64" | "arm64" => "aarch64".to_string(), // Handle both aarch64 and arm64
            "arm" => "aarch64".to_string(),
            _ => {
                tracing::warn!("Unknown architecture: {}, using as-is", raw_arch);
                raw_arch.to_lowercase()
            }
        };

        tracing::info!(
            "Detected platform: {}-{} (raw os: {}, raw arch: {})",
            os_name,
            arch,
            os,
            raw_arch
        );
        (os_name.to_string(), arch)
    }

    /// Convert command to use managed tools
    pub fn convert_command(&self, command: &str) -> String {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return command.to_string();
        }

        match parts[0] {
            "npx" => {
                // Convert npx to bun x
                let mut result = "bun".to_string();
                result.push(' ');
                result.push('x');
                for part in &parts[1..] {
                    result.push(' ');
                    result.push_str(part);
                }
                result
            }
                        "uvx" | "uv" | "npm" => {
                // Keep as is, will use managed tools
                command.to_string()
            }
            _ => command.to_string(),
        }
    }

    /// Get the executable path for a command
    pub async fn get_executable_path(&self, command: &str) -> Result<PathBuf> {
        let first_word = command.split_whitespace().next().unwrap_or("");

        match first_word {
            "npx" | "bun" => {
                // Handle both "npx" and "bun" commands
                // For "bun x" commands, we need to check if the second word is "x"
                let words: Vec<&str> = command.split_whitespace().collect();
                if words.len() > 1 && words[1] == "x" {
                    // This is "bun x", use managed bun executable
                    Ok(self.get_tool_path("bun"))
                } else {
                    // Regular "bun" command, use managed bun executable
                    Ok(self.get_tool_path("bun"))
                }
            },
            "uvx" => {
                // uvx uses the same executable as uv since they're in the same package
                if self.tool_exists("uvx").await {
                    Ok(self.get_tool_path("uvx"))
                } else {
                    // Fall back to uv executable if uvx doesn't exist as separate file
                    Ok(self.get_tool_path("uv"))
                }
            }
            "uv" => Ok(self.get_tool_path("uv")),
            "npm" => Ok(self.get_tool_path("bun")), // Use bun as npm replacement
            _ => {
                // For other commands, check if they exist in our bin directory
                let tool_path = self.get_tool_path(first_word);
                if self.tool_exists(first_word).await {
                    Ok(tool_path)
                } else {
                    // Fall back to system PATH
                    Err(McpError::ToolNotFound(first_word.to_string()))
                }
            }
        }
    }

    /// Ensure tools are installed for a command
    pub async fn ensure_tools_for_command(&self, command: &str) -> Result<()> {
        let first_word = command.split_whitespace().next().unwrap_or("");

        match first_word {
            "npx" | "bun" | "npm" => {
                // Check if this is "bun x" or regular bun command
                let words: Vec<&str> = command.split_whitespace().collect();
                let is_bun_x = words.len() > 1 && words[1] == "x";

                if !self.tool_exists("bun").await {
                    if is_bun_x {
                        tracing::info!("Installing Bun for bun x command");
                    } else {
                        tracing::info!("Installing Bun for bun command");
                    }
                    self.install_bun().await?;
                }
            }
            "uvx" | "uv" => {
                if !self.tool_exists("uv").await || !self.tool_exists("uvx").await {
                    tracing::info!("Installing UV and UVX");
                    self.install_uv_tools().await?;
                }
            }
            _ => {} // No special handling needed
        }

        Ok(())
    }

    /// Get all tools information
    pub async fn get_tools_info(&self) -> Result<Vec<crate::types::ToolInfo>> {
        let mut tools = Vec::new();

        // Bun tool
        let bun_path = self.get_tool_path("bun");
        let (bun_status, bun_version) = if self.tool_exists("bun").await {
            let version = self.get_tool_version("bun");
            (crate::types::ToolStatus::Installed, version)
        } else {
            (crate::types::ToolStatus::NotInstalled, None)
        };

        tools.push(crate::types::ToolInfo {
            name: "Bun".to_string(),
            full_name: "Bun JavaScript Runtime".to_string(),
            path: bun_path.to_string_lossy().to_string(),
            version: bun_version,
            status: bun_status,
            last_check: Some(chrono::Utc::now().to_rfc3339()),
            python_required: false,
        });

        // UV tool
        let uv_path = self.get_tool_path("uv");
        let (uv_status, uv_version) = if self.tool_exists("uv").await {
            let version = self.get_tool_version("uv");
            (crate::types::ToolStatus::Installed, version)
        } else {
            (crate::types::ToolStatus::NotInstalled, None)
        };

        tools.push(crate::types::ToolInfo {
            name: "UV".to_string(),
            full_name: "UV Package Manager".to_string(),
            path: uv_path.to_string_lossy().to_string(),
            version: uv_version.clone(),
            status: uv_status,
            last_check: Some(chrono::Utc::now().to_rfc3339()),
            python_required: true,
        });

        // UVX tool (separate executable from UV)
        let uvx_path = self.get_tool_path("uvx");
        let (uvx_status, uvx_version) = if self.tool_exists("uvx").await {
            // UVX uses same version as UV
            (crate::types::ToolStatus::Installed, uv_version.clone())
        } else {
            (crate::types::ToolStatus::NotInstalled, None)
        };

        tools.push(crate::types::ToolInfo {
            name: "UVX".to_string(),
            full_name: "UVX Package Executor".to_string(),
            path: uvx_path.to_string_lossy().to_string(),
            version: uvx_version,
            status: uvx_status,
            last_check: Some(chrono::Utc::now().to_rfc3339()),
            python_required: true,
        });

        // Update cached tools
        *self.tools.write().await = tools.clone();

        Ok(tools)
    }

    /// Check Python runtime compatibility
    pub async fn check_python_runtime(&self) -> Result<(bool, Option<String>)> {
        let output = TokioCommand::new("python3").arg("--version").output().await;

        match output {
            Ok(output) if output.status.success() => {
                let version_str = String::from_utf8_lossy(&output.stdout);
                let version = version_str.split_whitespace().last().map(|s| s.to_string());
                Ok((true, version))
            }
            _ => {
                // Try python command as fallback
                let output = TokioCommand::new("python").arg("--version").output().await;

                match output {
                    Ok(output) if output.status.success() => {
                        let version_str = String::from_utf8_lossy(&output.stdout);
                        let version = version_str.split_whitespace().last().map(|s| s.to_string());
                        Ok((true, version))
                    }
                    _ => Ok((false, None)),
                }
            }
        }
    }

    /// Install all required tools
    pub async fn install_all_tools(&self) -> Result<()> {
        tracing::info!("Installing all required tools...");

        // Check Python first
        let (python_available, python_version) = self.check_python_runtime().await?;
        if !python_available {
            tracing::warn!("Python not found, UV tools will not work properly");
        } else {
            tracing::info!("Python found: {:?}", python_version);
        }

        // Install Bun
        if let Err(e) = self.install_bun().await {
            tracing::error!("Failed to install Bun: {}", e);
            return Err(e);
        }

        // Install UV and UVX
        if python_available {
            if let Err(e) = self.install_uv_tools().await {
                tracing::error!("Failed to install UV tools: {}", e);
                return Err(e);
            }
        } else {
            tracing::warn!("Skipping UV tools installation due to missing Python");
        }

        tracing::info!("All tools installed successfully");
        Ok(())
    }

    /// Install a specific tool
    pub async fn install_tool(&self, tool_name: &str) -> Result<()> {
        tracing::info!("Installing tool: {}", tool_name);

        // Remove existing tool if it exists
        let tool_path = self.get_tool_path(tool_name);
        if tokio::fs::metadata(&tool_path).await.is_ok() {
            tokio::fs::remove_file(&tool_path).await?;
        }

        // Install based on tool type (case-insensitive)
        match tool_name.to_lowercase().as_str() {
            "bun" => {
                self.install_bun().await?;
                Ok(())
            }
            "uv" | "uvx" => {
                if let Ok((true, _)) = self.check_python_runtime().await {
                    self.install_uv_tools().await?;
                    Ok(())
                } else {
                    Err(McpError::RuntimeError("Python not available".to_string()))
                }
            }
            _ => Err(McpError::InvalidTool(tool_name.to_string())),
        }
    }

    /// Get a summary of tool status for startup check
    pub async fn get_startup_tool_status(&self) -> Result<crate::types::ToolStartupStatus> {
        let tools_info: Vec<crate::types::ToolInfo> = self.get_tools_info().await?;

        let bun_installed = tools_info
            .iter()
            .any(|t| t.name == "Bun" && t.status == crate::types::ToolStatus::Installed);
        let uv_installed = tools_info
            .iter()
            .any(|t| t.name == "UV" && t.status == crate::types::ToolStatus::Installed);
        let uvx_installed = tools_info
            .iter()
            .any(|t| t.name == "UVX" && t.status == crate::types::ToolStatus::Installed);

        let (python_available, _) = self.check_python_runtime().await?;

        Ok(crate::types::ToolStartupStatus {
            bun_installed,
            uv_installed,
            uvx_installed,
            python_available,
            missing_tools: tools_info
                .into_iter()
                .filter(|t| t.status != crate::types::ToolStatus::Installed)
                .map(|t| t.name)
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::tool_manager::ToolManager;

    #[test]
    fn test_convert_npx_to_bun_x() {
        let tool_manager = ToolManager::new();

        // Test npx command conversion
        let result = tool_manager.convert_command("npx create-react-app my-app");
        assert_eq!(result, "bun x create-react-app my-app");

        // Test npx with multiple arguments
        let result = tool_manager.convert_command("npx --package typescript tsc --version");
        assert_eq!(result, "bun x --package typescript tsc --version");

        // Test npx with quoted arguments
        let result = tool_manager.convert_command("npx \"my script\" --option");
        assert_eq!(result, "bun x \"my script\" --option");
    }

    #[test]
    fn test_convert_other_commands() {
        let tool_manager = ToolManager::new();

        // Test that other commands remain unchanged
        assert_eq!(tool_manager.convert_command("uv add numpy"), "uv add numpy");
        assert_eq!(tool_manager.convert_command("uvx run my-script"), "uvx run my-script");
        assert_eq!(tool_manager.convert_command("npm install"), "npm install");
        assert_eq!(tool_manager.convert_command("python script.py"), "python script.py");
    }

    #[test]
    fn test_convert_empty_command() {
        let tool_manager = ToolManager::new();
        let result = tool_manager.convert_command("");
        assert_eq!(result, "");
    }

    #[tokio::test]
    async fn test_get_executable_path_bun_x() {
        let tool_manager = ToolManager::new();

        // Test bun x command path
        let result = tool_manager.get_executable_path("bun x create-react-app").await;
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("bun"));
    }

    #[tokio::test]
    async fn test_get_executable_path_regular_bun() {
        let tool_manager = ToolManager::new();

        // Test regular bun command path
        let result = tool_manager.get_executable_path("bun run script.js").await;
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.to_string_lossy().contains("bun"));
    }

    #[test]
    fn test_npx_to_bun_x_conversion() {
        let tool_manager = ToolManager::new();

        // Test basic npx conversion
        let result = tool_manager.convert_command("npx create-react-app my-app");
        assert_eq!(result, "bun x create-react-app my-app");

        // Test npx with arguments including -y
        let result = tool_manager.convert_command("npx some-tool --option -y");
        assert_eq!(result, "bun x some-tool --option -y");

        // Test npx with quoted arguments
        let result = tool_manager.convert_command("npx \"my script\" --option");
        assert_eq!(result, "bun x \"my script\" --option");
    }

    #[test]
    fn test_command_parsing_logic() {
        let tool_manager = ToolManager::new();

        // Test the exact scenario from the log: "npx" with args ["-y", "@z_ai/mcp-server"]
        let command = "npx";
        let original_args = vec!["-y".to_string(), "@z_ai/mcp-server".to_string()];

        // Simulate the full command that would be passed to convert_command
        let full_command = format!("{} {}", command, original_args.join(" "));
        let converted_command = tool_manager.convert_command(&full_command);

        // Let's see what we actually get
        println!("Full command: {}", full_command);
        println!("Converted: {}", converted_command);

        // The converted command should be "bun x -y @z_ai/mcp-server"
        assert_eq!(converted_command, "bun x -y @z_ai/mcp-server");

        // Now test the filtering logic from mcp_client
        let first_word = command.split_whitespace().next().unwrap_or("");
        let final_args = if first_word == "npx" && converted_command.starts_with("bun x") {
            let converted_parts: Vec<&str> = converted_command.split_whitespace().collect();
            if converted_parts.len() >= 3 && converted_parts[0] == "bun" && converted_parts[1] == "x" {
                let mut filtered_args = Vec::new();
                for arg in converted_parts.iter() {
                    let arg_lower = arg.to_lowercase();
                    // Skip "bun" and -y/--yes, keep x and everything else
                    if *arg != "bun" && arg_lower != "-y" && arg_lower != "--yes" {
                        filtered_args.push(arg.to_string());
                    }
                }
                filtered_args
            } else {
                vec!["x".to_string()]
            }
        } else {
            original_args
        };

        // Expected result: ["x", "@z_ai/mcp-server"] (y should be filtered out)
        assert_eq!(final_args, vec!["x", "@z_ai/mcp-server"]);
        println!("✅ Command parsing test passed: {:?}", final_args);
    }

    #[test]
    fn test_npx_with_separate_args_conversion() {
        let tool_manager = ToolManager::new();

        // Test what happens when we pass "npx -y @z_ai/mcp-server" directly
        let converted = tool_manager.convert_command("npx -y @z_ai/mcp-server");
        println!("Direct conversion result: {}", converted);

        // This should be "bun x -y @z_ai/mcp-server"
        assert_eq!(converted, "bun x -y @z_ai/mcp-server");

        // Now filter it
        let converted_parts: Vec<&str> = converted.split_whitespace().collect();
        let mut filtered_args = Vec::new();
        for arg in converted_parts.iter() {
            let arg_lower = arg.to_lowercase();
            // Skip "bun" and "-y/--yes", keep "x" and everything else
            if *arg != "bun" && arg_lower != "-y" && arg_lower != "--yes" {
                filtered_args.push(arg.to_string());
            } else if *arg != "bun" {
                println!("Filtered out: {}", arg);
            }
        }

        assert_eq!(filtered_args, vec!["x", "@z_ai/mcp-server"]);
        println!("✅ Direct conversion test passed: {:?}", filtered_args);
    }

    #[test]
    fn test_remove_yes_arguments() {
        let command = "npx create-react-app my-app --yes";
        let converted_command = "bun x create-react-app my-app --yes";

        let original_first_word = command.split_whitespace().next().unwrap_or("");
        assert_eq!(original_first_word, "npx");

        if original_first_word == "npx" && converted_command.starts_with("bun x") {
            let converted_parts: Vec<&str> = converted_command.split_whitespace().collect();

            // Simulate the new filtering logic
            let mut final_args = vec!["x".to_string()];
            for arg in converted_parts[2..].iter() {
                let arg_lower = arg.to_lowercase();
                if arg_lower != "-y" && arg_lower != "--yes" {
                    final_args.push(arg.to_string());
                }
            }

            // --yes should be filtered out
            assert_eq!(final_args, vec!["x", "create-react-app", "my-app"]);
            println!("✅ Remove --yes test passed: {:?}", final_args);
        }
    }

    #[test]
    fn test_case_insensitive_yes_removal() {
        let test_cases = vec![
            ("npx tool -y", vec!["x", "tool"]),
            ("npx tool --yes", vec!["x", "tool"]),
            ("npx tool -Y", vec!["x", "tool"]),
            ("npx tool --YES", vec!["x", "tool"]),
            ("npx tool --option -y --yes", vec!["x", "tool", "--option"]),
        ];

        for (command, expected_args) in test_cases {
            let converted_command = command.replace("npx", "bun x");
            let converted_parts: Vec<&str> = converted_command.split_whitespace().collect();

            let mut final_args = vec!["x".to_string()];
            for arg in converted_parts[2..].iter() {
                let arg_lower = arg.to_lowercase();
                if arg_lower != "-y" && arg_lower != "--yes" {
                    final_args.push(arg.to_string());
                }
            }

            assert_eq!(final_args, expected_args, "Failed for command: {}", command);
            println!("✅ Case insensitive test passed for '{}': {:?}", command, final_args);
        }
    }

    #[test]
    fn test_non_npx_commands() {
        let tool_manager = ToolManager::new();

        // Test that non-npx commands remain unchanged
        assert_eq!(tool_manager.convert_command("uv add numpy"), "uv add numpy");
        assert_eq!(tool_manager.convert_command("uvx run my-script"), "uvx run my-script");
        assert_eq!(tool_manager.convert_command("npm install"), "npm install");
    }
}

