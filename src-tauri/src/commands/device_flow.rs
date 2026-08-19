//! GitHub 设备流登录命令

use crate::github_device_flow::{self, DeviceFlowPoll, DeviceFlowStart};
use super::response::ApiResponse;

/// 发起 GitHub 设备流登录，返回 user_code 与授权地址
#[tauri::command]
pub async fn github_device_login_start(client_id: String) -> ApiResponse<DeviceFlowStart> {
    match github_device_flow::start_device_flow(&client_id).await {
        Ok(start) => ApiResponse::success(start),
        Err(e) => ApiResponse::error(e.to_string()),
    }
}

/// 轮询设备流授权状态
#[tauri::command]
pub async fn github_device_login_poll(client_id: String, device_code: String) -> ApiResponse<DeviceFlowPoll> {
    match github_device_flow::poll_device_flow(&client_id, &device_code).await {
        Ok(poll) => ApiResponse::success(poll),
        Err(e) => ApiResponse::error(e.to_string()),
    }
}

/// 将设备流获取的 GitHub token 写入系统钥匙串（macOS），供 push/fetch 自动复用
#[tauri::command]
pub fn github_store_device_token(token: String) -> ApiResponse<String> {
    match github_device_flow::store_github_token(&token) {
        Ok(()) => ApiResponse::success("GitHub token stored to keychain".to_string()),
        Err(e) => ApiResponse::error(e.to_string()),
    }
}
