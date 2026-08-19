/**
 * GitHub 设备流登录 API
 *
 * 封装后端设备流命令：发起登录、轮询授权状态、存储 token 到钥匙串。
 */

import { invoke } from '@tauri-apps/api/core';

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

export interface DeviceFlowStart {
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

export interface DeviceFlowPoll {
  status: 'pending' | 'success' | 'error';
  token?: string;
  error?: string;
  error_description?: string;
}

/** 发起 GitHub 设备流登录 */
export async function startDeviceFlow(clientId: string): Promise<ApiResponse<DeviceFlowStart>> {
  try {
    return await invoke<ApiResponse<DeviceFlowStart>>('github_device_login_start', { clientId });
  } catch (error) {
    return { success: false, error: String(error) };
  }
}

/** 轮询设备流授权状态 */
export async function pollDeviceFlow(clientId: string, deviceCode: string): Promise<ApiResponse<DeviceFlowPoll>> {
  try {
    return await invoke<ApiResponse<DeviceFlowPoll>>('github_device_login_poll', { clientId, deviceCode });
  } catch (error) {
    return { success: false, error: String(error) };
  }
}

/** 将设备流获取的 token 存入系统钥匙串（macOS） */
export async function storeDeviceToken(token: string): Promise<ApiResponse<string>> {
  try {
    return await invoke<ApiResponse<string>>('github_store_device_token', { token });
  } catch (error) {
    return { success: false, error: String(error) };
  }
}
