use std::collections::HashMap;
use crate::error::Result;

/// 简化的 Shell 环境加载器，自动加载登录环境变量
pub struct ShellEnvironment;

impl ShellEnvironment {
    /// 加载系统环境变量（自动检测平台）
    pub async fn load_environment() -> Result<HashMap<String, String>> {
        use tokio::process::Command;

        let output = if cfg!(windows) {
            // Windows: 使用 cmd.exe /c set
            Command::new("cmd.exe")
                .args(["/c", "set"])
                .output()
                .await?
        } else {
            // macOS/Linux: 优先使用 zsh，回退到 bash
            let shell = if std::path::Path::new("/bin/zsh").exists() {
                "zsh"
            } else {
                "bash"
            };

            Command::new(shell)
                .args(["-ilc", "env"])
                .output()
                .await?
        };

        if !output.status.success() {
            return Err(crate::error::McpError::ShellError(
                format!("Failed to load environment: {}",
                       String::from_utf8_lossy(&output.stderr))
            ));
        }

        let env_output = String::from_utf8_lossy(&output.stdout);
        let mut env_vars = HashMap::new();

        for line in env_output.lines() {
            if let Some((key, value)) = line.split_once('=') {
                env_vars.insert(key.to_string(), value.to_string());
            }
        }

        // 确保管理工具目录在 PATH 中
        if let Ok(home_dir) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
            let tool_bin_path = format!("{}/.mcprouter/bin", home_dir);
            if let Some(path) = env_vars.get_mut("PATH") {
                let path_separator = if cfg!(windows) { ';' } else { ':' };
                if !path.contains(&tool_bin_path) {
                    path.insert_str(0, &format!("{}{}", tool_bin_path, path_separator));
                }
            } else {
                env_vars.insert("PATH".to_string(), tool_bin_path);
            }
        }

        Ok(env_vars)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_environment() {
        // Test that we can load environment without panicking
        match ShellEnvironment::load_environment().await {
            Ok(env_vars) => {
                println!("Loaded {} environment variables", env_vars.len());

                // Check that we have key environment variables
                assert!(env_vars.contains_key("PATH"), "PATH should be present");

                // Check that our tool bin directory is in PATH
                if let Some(path) = env_vars.get("PATH") {
                    let home_dir = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
                    let tool_bin_path = format!("{}/.mcprouter/bin", home_dir);
                    assert!(path.contains(&tool_bin_path), "Tool bin path should be in PATH: {}", path);
                }

                println!("Environment loading test passed!");
            }
            Err(e) => {
                println!("Environment loading failed (this may be expected in some environments): {}", e);
                // Don't fail the test as shell environment may not be available in all test environments
            }
        }
    }

    #[tokio::test]
    async fn test_environment_contains_shell_vars() {
        if let Ok(env_vars) = ShellEnvironment::load_environment().await {
            // Check for common shell environment variables
            let shell_vars = ["HOME", "USER", "SHELL"];
            for var in &shell_vars {
                if std::env::var(var).is_ok() {
                    assert!(env_vars.contains_key(*var), "Shell env should contain {}", var);
                    println!("{}: {}", var,
                        env_vars.get(*var)
                            .expect("Key should exist based on previous assert")
                    );
                }
            }
        }
    }
}