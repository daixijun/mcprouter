import { invoke } from '@tauri-apps/api/core'
import type { AppConfig, SystemSettings } from '../types'

class ServiceError extends Error {
  constructor(
    message: string,
    public readonly operation: string,
    public readonly cause?: Error,
  ) {
    super(message)
    this.name = 'ServiceError'
  }
}

// 添加超时包装器
function withTimeout<T>(
  promise: Promise<T>,
  timeoutMs: number = 10000,
  operation: string = '操作',
): Promise<T> {
  const timeoutPromise = new Promise<never>((_, reject) => {
    setTimeout(
      () =>
        reject(
          new ServiceError(`${operation}超时 (${timeoutMs}ms)`, operation),
        ),
      timeoutMs,
    )
  })

  return Promise.race([promise, timeoutPromise])
}

export class ConfigService {
  // Application Configuration
  static async getConfig(): Promise<AppConfig> {
    try {
      return await withTimeout(invoke('get_config'), 10000, '获取配置')
    } catch (error) {
      throw new ServiceError(
        '获取应用配置失败',
        'getConfig',
        error instanceof Error ? error : new Error(String(error)),
      )
    }
  }

  static async updateConfig(config: AppConfig): Promise<string> {
    try {
      return await withTimeout(
        invoke('update_config', { config }),
        15000,
        '更新配置',
      )
    } catch (error) {
      throw new ServiceError(
        '更新应用配置失败',
        'updateConfig',
        error instanceof Error ? error : new Error(String(error)),
      )
    }
  }

  // System Settings
  static async getSystemSettings(): Promise<SystemSettings> {
    try {
      return await withTimeout(invoke('get_settings'), 10000, '获取系统设置')
    } catch (error) {
      throw new ServiceError(
        '获取系统设置失败',
        'getSystemSettings',
        error instanceof Error ? error : new Error(String(error)),
      )
    }
  }

  static async saveSystemSettings(settings: SystemSettings): Promise<string> {
    try {
      return await withTimeout(
        invoke('save_settings', { settings }),
        15000,
        '保存系统设置',
      )
    } catch (error) {
      throw new ServiceError(
        '保存系统设置失败',
        'saveSystemSettings',
        error instanceof Error ? error : new Error(String(error)),
      )
    }
  }

  // Autostart Management
  // Note: To get autostart status, use the 'autostart' field from getSystemSettings() result
  static async toggleAutostart(): Promise<string> {
    try {
      return await withTimeout(invoke('toggle_autostart'), 10000, '切换自启动')
    } catch (error) {
      throw new ServiceError(
        '切换自启动失败',
        'toggleAutostart',
        error instanceof Error ? error : new Error(String(error)),
      )
    }
  }

  // Network Interface
  static async getLocalIpAddresses(): Promise<string[]> {
    try {
      return await withTimeout(
        invoke('get_local_ip_addresses'),
        5000,
        '获取本地IP地址',
      )
    } catch (error) {
      throw new ServiceError(
        '获取本地IP地址失败',
        'getLocalIpAddresses',
        error instanceof Error ? error : new Error(String(error)),
      )
    }
  }
}
