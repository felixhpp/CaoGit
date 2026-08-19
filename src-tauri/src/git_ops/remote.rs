//! Remote operations
//!
//! This module handles all remote-related Git operations (fetch, pull, push).

use git2::{Cred, RemoteCallbacks, PushOptions, FetchOptions};
use std::sync::{Arc, Mutex};
use anyhow::Result;
use tauri::Emitter;

use super::repository::GitRepository;
use super::types::{RemoteInfo, GitProgress, AuthConfig};

impl GitRepository {
    /// Fetch from a remote (without progress)
    #[allow(dead_code)]
    pub fn fetch(&self, remote_name: &str) -> Result<()> {
        let mut remote = self.repo.find_remote(remote_name)?;
        remote.fetch(&[] as &[&str], None, None)?;
        Ok(())
    }

    /// Fetch from a remote with progress reporting and timeout
    pub fn fetch_with_progress(&self, remote_name: &str, window: tauri::Window, auth_config: Option<AuthConfig>) -> Result<()> {
        let mut remote = self.repo.find_remote(remote_name)?;

        let mut callbacks = RemoteCallbacks::new();
        let window_clone = window.clone();
        let last_update = Arc::new(Mutex::new(std::time::Instant::now()));
        let start_time = Arc::new(std::time::Instant::now());
        let timeout_secs = 300; // 5 分钟超时

        callbacks.transfer_progress(move |stats| {
            // 检查超时
            if start_time.elapsed().as_secs() > timeout_secs {
                eprintln!("⏱️  Fetch operation timed out after {} seconds", timeout_secs);
                return false; // 中止操作
            }

            let mut last = match last_update.lock() {
                Ok(guard) => guard,
                Err(_) => return true, // Continue on lock error
            };
            let now = std::time::Instant::now();

            if now.duration_since(*last).as_millis() >= 200 {
                let received_bytes = stats.received_bytes();
                let total_objects = stats.total_objects();
                let received_objects = stats.received_objects();

                let duration = now.duration_since(*last).as_secs_f64();
                let speed = if duration > 0.0 {
                    (received_bytes as f64 / duration) as u64
                } else {
                    0
                };

                let progress = GitProgress {
                    operation_type: "download".to_string(),
                    total_objects,
                    received_objects,
                    total_bytes: received_bytes as u64,
                    received_bytes: received_bytes as u64,
                    speed_bytes_per_sec: speed,
                };

                let _ = window_clone.emit("git-progress", progress);
                *last = now;
            }

            true
        });

        // Clone auth_config for use in the callback
        let auth_config_clone = auth_config.clone();

        // 克隆仓库配置，用于调用系统凭据助手（git credential helper）
        let git_config = self.repo.config().ok();

        // 添加认证回调，支持 SSH 和 HTTPS
        callbacks.credentials(move |url, username_from_url, allowed_types| {
            // 0. 优先使用用户配置的认证信息
            if let Some(ref auth) = auth_config_clone {
                // Token 认证
                if auth.auth_type == "token" && allowed_types.is_user_pass_plaintext() {
                    if let Some(ref token) = auth.token {
                        let username = auth.username.as_deref().unwrap_or("git");
                        if let Ok(cred) = Cred::userpass_plaintext(username, token) {
                            return Ok(cred);
                        }
                    }
                }
                // 用户名/密码认证
                if auth.auth_type == "password" && allowed_types.is_user_pass_plaintext() {
                    if let (Some(ref username), Some(ref password)) = (&auth.username, &auth.password) {
                        if let Ok(cred) = Cred::userpass_plaintext(username, password) {
                            return Ok(cred);
                        }
                    }
                }

                if auth.auth_type == "ssh" && allowed_types.is_ssh_key() {
                    if let Some(ref private_key) = auth.ssh_key_path {
                        let username = username_from_url.unwrap_or("git");
                        let public_key = format!("{}.pub", private_key);
                        let public_key_path = std::path::Path::new(&public_key);
                        return Cred::ssh_key(
                            username,
                            public_key_path.exists().then_some(public_key_path),
                            std::path::Path::new(private_key),
                            None,
                        );
                    }
                }
            }

            // 1. 尝试 SSH agent
            if allowed_types.is_ssh_key() {
                if let Some(username) = username_from_url {
                    return Cred::ssh_key_from_agent(username);
                }
                return Cred::ssh_key_from_agent("git");
            }

            // 2. 尝试系统凭据（凭据助手 + macOS 钥匙串）
            if let Some(cred) = try_system_https_credentials(&git_config, url, username_from_url, allowed_types) {
                return Ok(cred);
            }

            // 3. 回退到默认凭据（用于 HTTPS）
            Cred::default()
        });

        let mut fetch_options = FetchOptions::new();
        fetch_options.remote_callbacks(callbacks);

        remote.fetch(&[] as &[&str], Some(&mut fetch_options), None)?;
        Ok(())
    }

    /// Pull from a remote (without progress)
    #[allow(dead_code)]
    pub fn pull(&self, remote_name: &str, branch_name: &str) -> Result<()> {
        self.fetch(remote_name)?;

        let fetch_head = self.repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = self.repo.reference_to_annotated_commit(&fetch_head)?;

        let (analysis, _) = self.repo.merge_analysis(&[&fetch_commit])?;

        if analysis.is_up_to_date() {
            return Ok(());
        } else if analysis.is_fast_forward() {
            let refname = format!("refs/heads/{}", branch_name);
            let mut reference = self.repo.find_reference(&refname)?;
            reference.set_target(fetch_commit.id(), "Fast-forward")?;
            self.repo.set_head(&refname)?;
            self.repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        } else {
            self.repo.merge(&[&fetch_commit], None, None)?;
        }

        Ok(())
    }

    /// Pull from a remote with progress reporting
    pub fn pull_with_progress(&self, remote_name: &str, branch_name: &str, window: tauri::Window, auth_config: Option<AuthConfig>) -> Result<()> {
        self.fetch_with_progress(remote_name, window, auth_config)?;

        let fetch_head = self.repo.find_reference("FETCH_HEAD")?;
        let fetch_commit = self.repo.reference_to_annotated_commit(&fetch_head)?;

        let (analysis, _) = self.repo.merge_analysis(&[&fetch_commit])?;

        if analysis.is_up_to_date() {
            return Ok(());
        } else if analysis.is_fast_forward() {
            let refname = format!("refs/heads/{}", branch_name);
            let mut reference = self.repo.find_reference(&refname)?;
            reference.set_target(fetch_commit.id(), "Fast-forward")?;
            self.repo.set_head(&refname)?;
            self.repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;
        } else {
            self.repo.merge(&[&fetch_commit], None, None)?;
        }

        Ok(())
    }

    /// Push to a remote (without progress)
    #[allow(dead_code)]
    pub fn push(&self, remote_name: &str, branch_name: &str, auth_config: Option<AuthConfig>) -> Result<()> {
        let mut remote = self.repo.find_remote(remote_name)?;
        let remote_url = remote.url().unwrap_or("unknown");
        eprintln!("📡 Push to remote: {} ({})", remote_name, remote_url);

        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);

        let mut callbacks = RemoteCallbacks::new();

        // 添加重试计数器，避免无限重试
        let retry_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let retry_count_clone = retry_count.clone();

        // Clone auth_config for use in the callback
        let auth_config_clone = auth_config.clone();

        // 克隆仓库配置，用于调用系统凭据助手（git credential helper）
        let git_config = self.repo.config().ok();

        // 添加详细的认证日志和多重回退机制
        callbacks.credentials(move |url, username_from_url, allowed_types| {
            let count = retry_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

            // 限制最多重试 3 次
            if count >= 3 {
                eprintln!("❌ 认证失败，已尝试 {} 次", count);
                eprintln!("   这通常意味着：");
                eprintln!("   1. 您的 SSH 密钥未在远程服务器上授权");
                eprintln!("   2. 或者 SSH agent 中的密钥与 GitHub 上的不匹配");
                eprintln!("   ");
                eprintln!("   请尝试运行：ssh -T git@github.com");
                return Err(git2::Error::from_str("认证失败，已尝试 3 次"));
            }

            eprintln!("🔐 请求凭据（第 {} 次尝试）：", count + 1);
            eprintln!("   URL: {}", url);
            eprintln!("   URL 中的用户名：{:?}", username_from_url);
            eprintln!("   允许的认证类型：{:?}", allowed_types);

            // 0. 优先使用用户配置的认证信息
            if let Some(ref auth) = auth_config_clone {
                eprintln!("   检测到用户配置的认证信息，类型：{}", auth.auth_type);

                // 如果是 token 认证且允许 HTTPS 认证
                if auth.auth_type == "token" && allowed_types.is_user_pass_plaintext() {
                    if let Some(ref token) = auth.token {
                        eprintln!("   正在使用配置的 Token 认证...");
                        // GitHub/GitLab 使用 token 作为密码，用户名可以是任意值（通常是 "git" 或实际用户名）
                        let username = auth.username.as_deref().unwrap_or("git");
                        match Cred::userpass_plaintext(username, token) {
                            Ok(cred) => {
                                eprintln!("   ✅ 已使用配置的 Token 创建凭据");
                                return Ok(cred);
                            }
                            Err(e) => {
                                eprintln!("   ❌ Token 认证失败：{}", e);
                            }
                        }
                    }
                }

                // 如果是用户名/密码认证且允许 HTTPS 认证
                if auth.auth_type == "password" && allowed_types.is_user_pass_plaintext() {
                    if let (Some(ref username), Some(ref password)) = (&auth.username, &auth.password) {
                        eprintln!("   正在使用配置的用户名/密码认证...");
                        match Cred::userpass_plaintext(username, password) {
                            Ok(cred) => {
                                eprintln!("   ✅ 已使用配置的用户名/密码创建凭据");
                                return Ok(cred);
                            }
                            Err(e) => {
                                eprintln!("   ❌ 用户名/密码认证失败：{}", e);
                            }
                        }
                    }
                }


                if auth.auth_type == "ssh" && allowed_types.is_ssh_key() {
                    if let Some(ref private_key) = auth.ssh_key_path {
                        eprintln!("   正在使用配置的 SSH 密钥...");
                        let username = username_from_url.unwrap_or("git");
                        let public_key = format!("{}.pub", private_key);
                        let public_key_path = std::path::Path::new(&public_key);
                        match Cred::ssh_key(
                            username,
                            public_key_path.exists().then_some(public_key_path),
                            std::path::Path::new(private_key),
                            None,
                        ) {
                            Ok(cred) => return Ok(cred),
                            Err(e) => eprintln!("   ❌ 配置的 SSH 密钥无法使用：{}", e),
                        }
                    }
                }
            }

            // 1. 尝试 SSH key from agent（最常用）
            if allowed_types.is_ssh_key() {
                eprintln!("   正在尝试从 SSH agent 获取密钥...");
                let username = username_from_url.unwrap_or("git");
                match Cred::ssh_key_from_agent(username) {
                    Ok(cred) => {
                        eprintln!("   ✅ 已从 SSH agent 获取凭据");
                        return Ok(cred);
                    }
                    Err(e) => {
                        eprintln!("   ❌ SSH agent 失败：{}", e);
                    }
                }
            }

            // 2. 尝试从默认位置读取 SSH 密钥
            if allowed_types.is_ssh_key() {
                eprintln!("   正在尝试从 ~/.ssh/id_rsa 读取密钥...");
                let username = username_from_url.unwrap_or("git");
                match std::env::var("HOME") {
                    Ok(home) => {
                        let private_key = format!("{}/.ssh/id_rsa", home);
                        let public_key = format!("{}/.ssh/id_rsa.pub", home);
                        match Cred::ssh_key(username, Some(std::path::Path::new(&public_key)), std::path::Path::new(&private_key), None) {
                            Ok(cred) => {
                                eprintln!("   ✅ 已从 ~/.ssh 获取密钥");
                                return Ok(cred);
                            }
                            Err(e) => {
                                eprintln!("   ❌ 从文件读取 SSH 密钥失败：{}", e);
                            }
                        }
                    }
                    Err(_) => {
                        eprintln!("   ❌ 未设置 HOME 环境变量");
                    }
                }
            }

            // 3. 尝试系统凭据（git 凭据助手 + macOS 钥匙串）
            if let Some(cred) = try_system_https_credentials(&git_config, url, username_from_url, allowed_types) {
                eprintln!("   ✅ 已获取系统凭据");
                return Ok(cred);
            }

            // 5. 尝试默认凭据（用于 HTTPS）
            if allowed_types.is_user_pass_plaintext() {
                eprintln!("   正在尝试默认凭据（HTTPS）...");
                match Cred::default() {
                    Ok(cred) => {
                        eprintln!("   ✅ 已获取默认凭据");
                        return Ok(cred);
                    }
                    Err(e) => {
                        eprintln!("   ❌ 默认凭据失败：{}", e);
                    }
                }
            }

            eprintln!("   ❌ 所有认证方法都失败了");
            Err(git2::Error::from_str("没有可用的有效认证方法"))
        });

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        eprintln!("🚀 Starting push operation...");
        remote.push(&[&refspec], Some(&mut push_options))?;
        eprintln!("✅ Push completed successfully");

        // Set upstream tracking after successful push
        let mut branch = self.repo.find_branch(branch_name, git2::BranchType::Local)?;
        branch.set_upstream(Some(&format!("{}/{}", remote_name, branch_name)))?;

        Ok(())
    }

    /// Push a tag to a remote
    pub fn push_tag(&self, remote_name: &str, tag_name: &str) -> Result<()> {
        let mut remote = self.repo.find_remote(remote_name)?;
        let refspec = format!("refs/tags/{}:refs/tags/{}", tag_name, tag_name);

        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            if let Some(username) = username_from_url {
                Cred::ssh_key_from_agent(username)
            } else {
                Cred::ssh_key_from_agent("git")
            }
        });

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        remote.push(&[&refspec], Some(&mut push_options))?;
        Ok(())
    }

    /// Push to a remote with progress reporting and timeout
    #[allow(dead_code)]
    pub fn push_with_progress(&self, remote_name: &str, branch_name: &str, window: tauri::Window) -> Result<()> {
        let mut remote = self.repo.find_remote(remote_name)?;

        let mut callbacks = RemoteCallbacks::new();
        let start_time = Arc::new(std::time::Instant::now());
        let timeout_secs = 300; // 5 分钟超时

        callbacks.credentials(|_url, username_from_url, allowed_types| {
            // 优先尝试 SSH agent
            if allowed_types.is_ssh_key() {
                if let Some(username) = username_from_url {
                    return Cred::ssh_key_from_agent(username);
                }
                return Cred::ssh_key_from_agent("git");
            }
            // 回退到默认凭据（用于 HTTPS）
            Cred::default()
        });

        let window_clone = window.clone();
        let last_update = Arc::new(Mutex::new(std::time::Instant::now()));
        let last_bytes = Arc::new(Mutex::new(0usize));

        callbacks.push_transfer_progress(move |current, total, bytes| {
            // 检查超时
            if start_time.elapsed().as_secs() > timeout_secs {
                eprintln!("⏱️  Push operation timed out after {} seconds", timeout_secs);
                // push_transfer_progress 没有返回值，无法中止
                // 但可以停止发送进度更新
                return;
            }

            let mut last = match last_update.lock() {
                Ok(guard) => guard,
                Err(_) => return, // Continue on lock error
            };
            let mut prev_bytes = match last_bytes.lock() {
                Ok(guard) => guard,
                Err(_) => return, // Continue on lock error
            };
            let now = std::time::Instant::now();

            if now.duration_since(*last).as_millis() >= 200 {
                let duration = now.duration_since(*last).as_secs_f64();
                let bytes_diff = bytes.saturating_sub(*prev_bytes);
                let speed = if duration > 0.0 {
                    (bytes_diff as f64 / duration) as u64
                } else {
                    0
                };

                let progress = GitProgress {
                    operation_type: "upload".to_string(),
                    total_objects: total,
                    received_objects: current,
                    total_bytes: bytes as u64,
                    received_bytes: bytes as u64,
                    speed_bytes_per_sec: speed,
                };

                let _ = window_clone.emit("git-progress", progress);
                *last = now;
                *prev_bytes = bytes;
            }
        });

        let mut push_options = PushOptions::new();
        push_options.remote_callbacks(callbacks);

        let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
        remote.push(&[&refspec], Some(&mut push_options))?;

        // Set upstream tracking after successful push
        let mut branch = self.repo.find_branch(branch_name, git2::BranchType::Local)?;
        branch.set_upstream(Some(&format!("{}/{}", remote_name, branch_name)))?;

        Ok(())
    }

    /// Get all remotes
    pub fn get_remotes(&self) -> Result<Vec<RemoteInfo>> {
        let remotes = self.repo.remotes()?;
        let mut remote_infos = Vec::new();

        for name in remotes.iter() {
            if let Some(name) = name {
                if let Ok(remote) = self.repo.find_remote(name) {
                    if let Some(url) = remote.url() {
                        remote_infos.push(RemoteInfo {
                            name: name.to_string(),
                            url: url.to_string(),
                        });
                    }
                }
            }
        }

        Ok(remote_infos)
    }

    /// Get the URL of a specific remote
    pub fn get_remote_url(&self, name: &str) -> Result<String> {
        let remote = self.repo.find_remote(name)?;
        remote
            .url()
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Remote '{}' has no URL", name))
    }

    /// Add a new remote
    pub fn add_remote(&self, name: &str, url: &str) -> Result<()> {
        self.repo.remote(name, url)?;
        Ok(())
    }

    /// Remove a remote
    pub fn remove_remote(&self, name: &str) -> Result<()> {
        self.repo.remote_delete(name)?;
        Ok(())
    }
}

/// 尝试获取系统级 HTTPS 凭据：先走 git 凭据助手（credential.helper），
/// 再回退到 macOS 钥匙串直读。返回可用凭据则直接使用。
fn try_system_https_credentials(
    git_config: &Option<git2::Config>,
    url: &str,
    username_from_url: Option<&str>,
    allowed_types: git2::CredentialType,
) -> Option<git2::Cred> {
    if !allowed_types.is_user_pass_plaintext() {
        return None;
    }

    // 1. git 凭据助手（需 git 配置可达且 helper 可执行）
    if let Some(ref config) = git_config {
        if let Ok(cred) = Cred::credential_helper(config, url, username_from_url) {
            return Some(cred);
        }
    }

    // 2. 设备流登录写入的 GitHub token（generic password，账户 github_token），仅用于 GitHub 主机
    #[cfg(target_os = "macos")]
    if url.contains("github.com") {
        if let Ok(token) = crate::keychain::get_password(crate::keychain::GITHUB_TOKEN_ACCOUNT) {
            if !token.is_empty() {
                if let Ok(cred) = Cred::userpass_plaintext("git", &token) {
                    return Some(cred);
                }
            }
        }
    }

    // 3. macOS 钥匙串直读（等价于 git credential osxkeychain）
    #[cfg(target_os = "macos")]
    if let Some((account, password)) = read_keychain_https_credentials(url) {
        if let Ok(cred) = Cred::userpass_plaintext(&account, &password) {
            return Some(cred);
        }
    }

    None
}

/// 从 macOS 系统钥匙串读取 HTTPS 凭据（等价于 git credential osxkeychain helper）。
/// 返回 (账户名, 密码/Token)。仅限 macOS 使用。
#[cfg(target_os = "macos")]
fn read_keychain_https_credentials(url: &str) -> Option<(String, String)> {
    use std::process::Command;

    // 从远程 URL 提取主机名，如 https://github.com/owner/repo.git -> github.com
    let host = url
        .split("://")
        .nth(1)?
        .split('@')
        .last()?
        .split('/')
        .next()?
        .split(':')
        .next()?
        .to_string();
    if host.is_empty() {
        return None;
    }

    // 读取账户名（如 "acct"<blob>="felixhpp"）
    let acct_output = Command::new("security")
        .args(["find-internet-password", "-s", &host])
        .output()
        .ok()?;
    if !acct_output.status.success() {
        return None;
    }
    let acct_text = String::from_utf8_lossy(&acct_output.stdout);
    let account = acct_text.lines().find_map(|line| {
        let line = line.trim();
        let (_, rest) = line.split_once("\"acct\"")?;
        let start = rest.find('"')? + 1;
        let end = rest[start..].find('"')? + start;
        Some(rest[start..end].to_string())
    }).unwrap_or_else(|| "git".to_string());

    // 读取密码（Token），-w 仅输出密码本身
    let pass_output = Command::new("security")
        .args(["find-internet-password", "-s", &host, "-w"])
        .output()
        .ok()?;
    if !pass_output.status.success() {
        return None;
    }
    let password = String::from_utf8_lossy(&pass_output.stdout).trim().to_string();
    if password.is_empty() {
        return None;
    }

    Some((account, password))
}
