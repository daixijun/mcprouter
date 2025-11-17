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
  operation: string = 'Operation',
): Promise<T> {
  const timeoutPromise = new Promise<never>((_, reject) => {
    setTimeout(
      () =>
        reject(
          new ServiceError(`Operation timeout (${timeoutMs}ms)`, operation),
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
      return await withTimeout(invoke('get_config'), 10000, 'Get config')
    } catch (error) {
      throw new ServiceError(
        'Failed to get application configuration',
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
        'Update config',
      )
    } catch (error) {
      throw new ServiceError(
        'Failed to update application configuration',
        'updateConfig',
        error instanceof Error ? error : new Error(String(error)),
      )
    }
  }

  // System Settings
  static async getSystemSettings(): Promise<SystemSettings> {
    try {
      return await withTimeout(
        invoke('get_settings'),
        10000,
        'Get system settings',
      )
    } catch (error) {
      throw new ServiceError(
        'Failed to get system settings',
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
        'Save system settings',
      )
    } catch (error) {
      throw new ServiceError(
        'Failed to save system settings',
        'saveSystemSettings',
        error instanceof Error ? error : new Error(String(error)),
      )
    }
  }

  // Autostart Management
  // Note: To get autostart status, use the 'autostart' field from getSystemSettings() result
  static async toggleAutostart(): Promise<string> {
    try {
      return await withTimeout(
        invoke('toggle_autostart'),
        10000,
        'Toggle autostart',
      )
    } catch (error) {
      throw new ServiceError(
        'Failed to toggle autostart',
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
        'Get local IP addresses',
      )
    } catch (error) {
      throw new ServiceError(
        'Failed to get local IP addresses',
        'getLocalIpAddresses',
        error instanceof Error ? error : new Error(String(error)),
      )
    }
  }
}
