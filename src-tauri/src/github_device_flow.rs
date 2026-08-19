//! GitHub 设备授权流（Device Flow）登录
//!
//! 流程参考 GitHub OAuth Device Flow 规范：
//! 1. 调用 `POST https://github.com/login/device/code` 获取 user_code 与 verification_uri
//! 2. 用户在浏览器打开 verification_uri 并输入 user_code 完成授权
//! 3. 轮询 `POST https://github.com/login/oauth/access_token` 获取 access_token
//! 4. 将 token 写入系统钥匙串，供 git 操作（push/fetch）自动复用

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

const DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const ACCESS_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const DEFAULT_SCOPE: &str = "repo"; // 仓库读写权限（含私有仓库）

/// 设备流初始化结果，前端需向用户展示 user_code 并引导打开 verification_uri
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceFlowStart {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

/// 轮询结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeviceFlowPoll {
    /// "pending" | "success" | "error"
    pub status: String,
    pub token: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

/// 发起设备流登录，返回需要用户授权的验证码与验证地址
pub async fn start_device_flow(client_id: &str) -> Result<DeviceFlowStart> {
    let client = reqwest::Client::new();
    let resp = client
        .post(DEVICE_CODE_URL)
        .header("Accept", "application/json")
        .form(&[("client_id", client_id), ("scope", DEFAULT_SCOPE)])
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        return Err(anyhow!("GitHub 设备流初始化失败（HTTP {}）：{}", status, text));
    }
    let start: DeviceFlowStart = serde_json::from_str(&text)
        .map_err(|e| anyhow!("解析设备流初始化响应失败：{}，响应：{}", e, text))?;
    Ok(start)
}

/// 轮询设备流授权状态，直到成功、失败或仍在等待
pub async fn poll_device_flow(client_id: &str, device_code: &str) -> Result<DeviceFlowPoll> {
    let client = reqwest::Client::new();
    let resp = client
        .post(ACCESS_TOKEN_URL)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id),
            ("device_code", device_code),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
        ])
        .send()
        .await?;
    let status = resp.status();
    let text = resp.text().await?;

    let json: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(_) => return Err(anyhow!("GitHub 设备流轮询响应异常（HTTP {}）：{}", status, text)),
    };

    // 授权成功：返回 access_token
    if let Some(token) = json.get("access_token").and_then(|v| v.as_str()) {
        return Ok(DeviceFlowPoll {
            status: "success".to_string(),
            token: Some(token.to_string()),
            error: None,
            error_description: None,
        });
    }

    // 处理规范中的错误码
    if let Some(err) = json.get("error").and_then(|v| v.as_str()) {
        let desc = json
            .get("error_description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        match err {
            // 用户尚未授权，继续轮询
            "authorization_pending" | "slow_down" => Ok(DeviceFlowPoll {
                status: "pending".to_string(),
                token: None,
                error: None,
                error_description: None,
            }),
            // 明确失败：过期或被拒绝
            "expired_token" | "access_denied" => Ok(DeviceFlowPoll {
                status: "error".to_string(),
                token: None,
                error: Some(err.to_string()),
                error_description: Some(desc),
            }),
            _ => Err(anyhow!("GitHub 设备流轮询失败（HTTP {}）：{} {}", status, err, desc)),
        }
    } else {
        Err(anyhow!("GitHub 设备流轮询失败（HTTP {}）：{}", status, text))
    }
}

/// 将设备流获取的 token 写入系统钥匙串（复用 keychain 模块的 generic password，账户 github_token）
pub fn store_github_token(token: &str) -> Result<()> {
    crate::keychain::save_password(crate::keychain::GITHUB_TOKEN_ACCOUNT, token)
}
