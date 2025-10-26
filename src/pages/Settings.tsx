import { listen } from '@tauri-apps/api/event'
import React, { useEffect, useState } from 'react'
import toastService from '../services/toastService'
import type { SystemSettings } from '../types'

const Settings: React.FC = () => {
  const [settings, setSettings] = useState<SystemSettings>({
    server: {
      host: 'localhost',
      port: 8850,
      max_connections: 100,
      timeout_seconds: 30,
    },
    logging: {
      level: 'info',
      file_name: '',
    },
    security: {
      auth: true,
      allowed_hosts: ['localhost', '127.0.0.1'],
    },
    // 应用层设置（默认值，仅用于初始渲染；实际以后台配置为准）
    settings: {
      theme: 'auto',
      autostart: false,
      system_tray: {
        enabled: true,
        close_to_tray: false,
        start_to_tray: false,
      },
    },
  })

  const [loading, setLoading] = useState(false)
  const [saving, setSaving] = useState(false)
  const [activeTab, setActiveTab] = useState<
    'server' | 'logging' | 'security' | 'application' | 'tools'
  >('server')
  const [autostartEnabled, setAutostartEnabled] = useState(false)
  const [localIpAddresses, setLocalIpAddresses] = useState<string[]>([])

  useEffect(() => {
    const loadData = async () => {
      try {
        // 并行加载设置、IP地址和自动启动状态
        await Promise.all([
          loadSettings(),
          loadLocalIpAddresses(),
          loadAutostartStatus(),
        ])
      } catch (error) {
        console.error('Failed to load initial data:', error)
        // 错误已经在各自的加载函数中处理，这里只需要记录
      }
    }

    loadData()
  }, [])

  // 监听主题变更事件，保持设置页的 settings.theme 同步
  useEffect(() => {
    let cleanup: (() => void) | undefined
    listen<string>('theme-changed', (event) => {
      const newTheme = event.payload as 'light' | 'dark' | 'auto'
      setSettings((prev) => ({
        ...prev,
        settings: {
          ...(prev.settings || {}),
          theme: newTheme,
        },
      }))
    })
      .then((unlisten) => {
        cleanup = unlisten
      })
      .catch((error) => {
        console.error('Failed to setup theme listener in Settings:', error)
      })
    return () => {
      cleanup && cleanup()
    }
  }, [])

  const loadAutostartStatus = async () => {
    try {
      const { ApiService } = await import('../services/api')
      const enabled = await ApiService.isAutostartEnabled()
      setAutostartEnabled(enabled)
    } catch (error) {
      console.error('Failed to load autostart status:', error)
    }
  }

  const loadLocalIpAddresses = async () => {
    try {
      // 导入API服务
      const { ApiService } = await import('../services/api')

      // 调用后端API获取本机IP地址列表
      const ips = await ApiService.getLocalIpAddresses()

      // 更新IP地址状态
      setLocalIpAddresses(ips)
    } catch (error) {
      console.error('Failed to load local IP addresses:', error)

      // 显示错误通知，不设置默认值
      toastService.sendErrorNotification(
        `加载本机IP地址失败: ${
          error instanceof Error ? error.message : '未知错误'
        }`,
      )

      // 抛出错误，让调用者处理
      throw error
    }
  }

  const loadSettings = async () => {
    setLoading(true)
    try {
      // 导入API服务
      const { ApiService } = await import('../services/api')

      // 调用后端API加载设置
      const loadedSettings = await ApiService.getSystemSettings()

      // 更新设置状态
      setSettings(loadedSettings)
    } catch (error) {
      console.error('Failed to load settings:', error)
      toastService.sendErrorNotification(
        '加载设置失败，请检查网络连接或稍后重试。',
      )

      // 显示错误通知，不使用默认设置
      toastService.sendErrorNotification(
        `加载系统设置失败: ${
          error instanceof Error ? error.message : '未知错误'
        }`,
      )

      // 抛出错误，让调用者处理
      throw error
    } finally {
      setLoading(false)
    }
  }

  const saveSettings = async () => {
    setSaving(true)
    try {
      // 导入API服务
      const { ApiService } = await import('../services/api')

      // 调用后端API保存设置
      await ApiService.saveSystemSettings(settings)

      // 显示成功通知
      toastService.sendSuccessNotification('设置保存成功！系统配置已更新。')
    } catch (error) {
      console.error('Failed to save settings:', error)
      toastService.sendErrorNotification(
        '保存设置失败，请检查网络连接或稍后重试。',
      )
    } finally {
      setSaving(false)
    }
  }

  const handleServerSettingChange = (key: string, value: any) => {
    setSettings((prev) => ({
      ...prev,
      server: {
        ...prev.server,
        [key]: value,
      },
    }))
  }

  const handleLoggingSettingChange = (key: string, value: any) => {
    setSettings((prev) => ({
      ...prev,
      logging: {
        ...prev.logging,
        [key]: value,
      },
    }))
  }

  const handleSecuritySettingChange = (key: string, value: any) => {
    setSettings((prev) => ({
      ...prev,
      security: {
        ...prev.security,
        [key]: value,
      },
    }))
  }

  const addAllowedHost = () => {
    const newHost = prompt('请输入允许的主机地址（如：192.168.1.100）:')
    if (newHost) {
      setSettings((prev) => ({
        ...prev,
        security: {
          ...prev.security,
          allowed_hosts: [...prev.security.allowed_hosts, newHost],
        },
      }))
    }
  }

  const removeAllowedHost = (index: number) => {
    setSettings((prev) => ({
      ...prev,
      security: {
        ...prev.security,
        allowed_hosts: prev.security.allowed_hosts.filter(
          (_, i) => i !== index,
        ),
      },
    }))
  }

  // 托盘相关：切换托盘开关（立即保存）
  const toggleSystemTrayEnabled = async () => {
    try {
      const { ApiService } = await import('../services/api')
      setSettings((prev) => {
        const prevEnabled = prev.settings?.system_tray?.enabled ?? true
        const nextEnabled = !prevEnabled
        const nextSystemTray = {
          enabled: nextEnabled,
          close_to_tray: nextEnabled
            ? prev.settings?.system_tray?.close_to_tray ?? false
            : false,
          start_to_tray: nextEnabled
            ? prev.settings?.system_tray?.start_to_tray ?? false
            : false,
        }
        const next = {
          ...prev,
          settings: {
            ...prev.settings,
            system_tray: nextSystemTray,
          },
        }
        // 立即保存并生效：提交完整 settings 对象，后端会解析嵌套结构
        ApiService.saveSystemSettings(next)
          .then(() => {
            toastService.sendSuccessNotification(
              nextEnabled
                ? '系统托盘已启用并立即生效'
                : '系统托盘已禁用并立即移除',
            )
          })
          .catch((error) => {
            console.error('Failed to apply system tray toggle:', error)
            toastService.sendErrorNotification('切换系统托盘失败，请稍后重试')
          })
        return next
      })
    } catch (error) {
      console.error('Failed to toggle system tray:', error)
      toastService.sendErrorNotification('切换系统托盘失败')
    }
  }

  // 托盘相关：切换关闭时最小化到托盘（立即保存）
  const toggleMinimizeOnClose = async () => {
    try {
      const { ApiService } = await import('../services/api')
      setSettings((prev) => {
        const nextClose = !(prev.settings?.system_tray?.close_to_tray ?? false)
        const next = {
          ...prev,
          settings: {
            ...prev.settings,
            system_tray: {
              ...prev.settings?.system_tray,
              close_to_tray: nextClose,
            },
          },
        }
        ApiService.saveSystemSettings(next)
          .then(() => {
            toastService.sendSuccessNotification(
              nextClose ? '关闭窗口将最小化到托盘' : '关闭窗口不再最小化到托盘',
            )
          })
          .catch((error) => {
            console.error('Failed to apply minimize-on-close toggle:', error)
            toastService.sendErrorNotification('切换“关闭时最小化到托盘”失败')
          })
        return next
      })
    } catch (error) {
      console.error('Failed to toggle minimize on close:', error)
      toastService.sendErrorNotification('切换“关闭时最小化到托盘”失败')
    }
  }

  // 托盘相关：切换启动时最小化到托盘（立即保存）
  const toggleMinimizeOnStart = async () => {
    try {
      const { ApiService } = await import('../services/api')
      setSettings((prev) => {
        const nextStart = !(prev.settings?.system_tray?.start_to_tray ?? false)
        const next = {
          ...prev,
          settings: {
            ...prev.settings,
            system_tray: {
              ...prev.settings?.system_tray,
              start_to_tray: nextStart,
            },
          },
        }
        ApiService.saveSystemSettings(next)
          .then(() => {
            toastService.sendSuccessNotification(
              nextStart ? '启动时将最小化到托盘' : '启动时不再最小化到托盘',
            )
          })
          .catch((error) => {
            console.error('Failed to apply minimize-on-start toggle:', error)
            toastService.sendErrorNotification('切换“启动时最小化到托盘”失败')
          })
        return next
      })
    } catch (error) {
      console.error('Failed to toggle minimize on start:', error)
      toastService.sendErrorNotification('切换“启动时最小化到托盘”失败')
    }
  }

  // 自动启动：保持原逻辑（调用后端命令），并刷新状态
  const toggleAutostart = async () => {
    try {
      const { ApiService } = await import('../services/api')
      const result = await ApiService.toggleAutostart()
      toastService.sendSuccessNotification(result)
      await loadAutostartStatus()
    } catch (error) {
      console.error('Failed to toggle autostart:', error)
      toastService.sendErrorNotification('切换自动启动失败')
    }
  }

  if (loading) {
    return (
      <div className='flex items-center justify-center h-64'>
        <div className='animate-spin rounded-full h-12 w-12 border-4 border-blue-500 border-t-transparent'></div>
      </div>
    )
  }

  return (
    <div className='h-full flex flex-col space-y-6 compact-container overflow-y-auto'>
      {/* Tab Navigation */}
      <div className='flex flex-wrap gap-2 mb-8'>
        <button
          onClick={() => setActiveTab('server')}
          className={`px-4 py-2 rounded-lg font-medium transition-all ${
            activeTab === 'server'
              ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-lg'
              : 'bg-white/10 text-gray-700 hover:bg-white/20 dark:text-gray-300 dark:hover:text-white'
          }`}>
          🖥️ 服务器配置
        </button>
        <button
          onClick={() => setActiveTab('logging')}
          className={`px-4 py-2 rounded-lg font-medium transition-all ${
            activeTab === 'logging'
              ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-lg'
              : 'bg-white/10 text-gray-700 hover:bg-white/20 dark:text-gray-300 dark:hover:text-white'
          }`}>
          📝 日志配置
        </button>
        <button
          onClick={() => setActiveTab('security')}
          className={`px-4 py-2 rounded-lg font-medium transition-all ${
            activeTab === 'security'
              ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-lg'
              : 'bg-white/10 text-gray-700 hover:bg-white/20 dark:text-gray-300 dark:hover:text-white'
          }`}>
          🔒 安全设置
        </button>
        <button
          onClick={() => setActiveTab('application')}
          className={`px-4 py-2 rounded-lg font-medium transition-all ${
            activeTab === 'application'
              ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-lg'
              : 'bg-white/10 text-gray-700 hover:bg-white/20 dark:text-gray-300 dark:hover:text-white'
          }`}>
          ⚙️ 应用设置
        </button>
        <button
          onClick={() => setActiveTab('tools')}
          className={`px-4 py-2 rounded-lg font-medium transition-all ${
            activeTab === 'tools'
              ? 'bg-white dark:bg-gray-700 text-gray-900 dark:text-gray-100 shadow-lg'
              : 'bg-white/10 text-gray-700 hover:bg-white/20 dark:text-gray-300 dark:hover:text-white'
          }`}>
          🧰 工具设置
        </button>
      </div>

      <div className='card-glass p-6'>
        {/* Server Settings Tab */}
        {activeTab === 'server' && (
          <div className='space-y-6'>
            <div>
              <h3 className='text-xl font-semibold text-gray-800 dark:text-gray-100 mb-4'>
                🖥️ 服务器配置
              </h3>

              <div className='grid grid-cols-1 md:grid-cols-2 gap-6'>
                <div>
                  <div className='flex items-center gap-4'>
                    <label className='text-sm font-medium text-gray-700 dark:text-gray-300 w-32 flex-shrink-0'>
                      监听主机
                    </label>
                    <select
                      value={settings.server.host}
                      onChange={(e) =>
                        handleServerSettingChange('host', e.target.value)
                      }
                      className='input-modern flex-1'>
                      {localIpAddresses.map((ip) => {
                        let description = ''
                        if (ip === 'localhost' || ip === '127.0.0.1') {
                          description = ' (仅本机访问)'
                        } else if (ip === '0.0.0.0') {
                          description = ' (所有网络接口)'
                        } else if (
                          ip.startsWith('192.168.') ||
                          ip.startsWith('10.') ||
                          ip.startsWith('172.')
                        ) {
                          description = ' (局域网访问)'
                        }

                        return (
                          <option key={ip} value={ip}>
                            {ip}
                            {description}
                          </option>
                        )
                      })}
                    </select>
                  </div>
                  <p className='text-xs text-gray-500 mt-1 ml-36'>
                    服务器绑定的主机地址
                  </p>
                </div>
                <div>
                  <div className='flex items-center gap-4'>
                    <label className='text-sm font-medium text-gray-700 dark:text-gray-300 w-32 flex-shrink-0'>
                      MCP服务监听端口
                    </label>
                    <input
                      type='number'
                      value={settings.server.port}
                      onChange={(e) =>
                        handleServerSettingChange(
                          'port',
                          parseInt(e.target.value),
                        )
                      }
                      className='input-modern flex-1'
                      min='1'
                      max='65535'
                    />
                  </div>
                  <p className='text-xs text-gray-500 mt-1 ml-36'>
                    MCP聚合服务器监听的端口号
                  </p>
                </div>

                <div>
                  <div className='flex items-center gap-4'>
                    <label className='text-sm font-medium text-gray-700 dark:text-gray-300 w-32 flex-shrink-0'>
                      最大连接数
                    </label>
                    <input
                      type='number'
                      value={settings.server.max_connections}
                      onChange={(e) =>
                        handleServerSettingChange(
                          'max_connections',
                          parseInt(e.target.value),
                        )
                      }
                      className='input-modern flex-1'
                      min='1'
                      max='10000'
                    />
                  </div>
                  <p className='text-xs text-gray-500 mt-1 ml-36'>
                    同时允许的最大客户端连接数
                  </p>
                </div>

                <div>
                  <div className='flex items-center gap-4'>
                    <label className='text-sm font-medium text-gray-700 dark:text-gray-300 w-32 flex-shrink-0'>
                      超时时间（秒）
                    </label>
                    <input
                      type='number'
                      value={settings.server.timeout_seconds}
                      onChange={(e) =>
                        handleServerSettingChange(
                          'timeout_seconds',
                          parseInt(e.target.value),
                        )
                      }
                      className='input-modern flex-1'
                      min='1'
                      max='300'
                    />
                  </div>
                  <p className='text-xs text-gray-500 mt-1 ml-36'>
                    客户端请求超时时间
                  </p>
                </div>
              </div>
            </div>
          </div>
        )}

        {/* Logging Settings Tab */}
        {activeTab === 'logging' && (
          <div className='space-y-6'>
            <h3 className='text-xl font-semibold text-gray-800 dark:text-gray-100 mb-4'>
              📝 日志配置
            </h3>

            <div className='grid grid-cols-1 md:grid-cols-2 gap-6'>
              <div>
                <div className='flex items-center gap-4'>
                  <label className='text-sm font-medium text-gray-700 dark:text-gray-300 w-32 flex-shrink-0'>
                    日志级别
                  </label>
                  <select
                    value={settings.logging.level}
                    onChange={(e) =>
                      handleLoggingSettingChange('level', e.target.value)
                    }
                    className='input-modern flex-1'>
                    <option value='trace'>TRACE (跟踪)</option>
                    <option value='debug'>DEBUG (调试)</option>
                    <option value='info'>INFO (信息)</option>
                    <option value='warn'>WARN (警告)</option>
                    <option value='error'>ERROR (错误)</option>
                  </select>
                </div>
                <p className='text-xs text-gray-500 mt-1 ml-36'>
                  系统日志记录级别
                </p>
              </div>

              <div>
                <div className='flex items-center gap-4'>
                  <label className='text-sm font-medium text-gray-700 dark:text-gray-300 w-32 flex-shrink-0'>
                    日志文件名
                  </label>
                  <input
                    type='text'
                    value={settings.logging.file_name || ''}
                    onChange={(e) =>
                      handleLoggingSettingChange('file_name', e.target.value)
                    }
                    className='input-modern flex-1'
                    placeholder='mcprouter.log'
                  />
                </div>
                <p className='text-xs text-gray-500 mt-1 ml-36'>
                  日志文件名（留空使用默认名称）
                </p>
              </div>
            </div>

            <div className='bg-blue-50 dark:bg-blue-900/20 border border-blue-200 dark:border-blue-800 rounded-lg p-4 mt-4'>
              <p className='text-sm text-blue-800'>
                💡 <strong>日志输出位置：</strong>
              </p>
              <ul className='text-sm text-blue-700 mt-2 space-y-1 ml-4'>
                <li>• 终端：日志会实时输出到应用运行的终端窗口</li>
                <li>• 日志文件：自动保存到系统日志目录</li>
                <li className='text-xs mt-2 opacity-75'>
                  macOS: ~/Library/Logs/mcprouter/ | Linux:
                  ~/.config/mcprouter/logs/ | Windows: %APPDATA%\mcprouter\logs\
                </li>
              </ul>
            </div>
          </div>
        )}

        {/* Security Settings Tab */}
        {activeTab === 'security' && (
          <div className='space-y-6'>
            <h3 className='text-xl font-semibold text-gray-800 dark:text-gray-100 mb-4'>
              🔒 安全设置
            </h3>

            <div className='space-y-4'>
              <div className='flex items-center'>
                <input
                  type='checkbox'
                  checked={!!settings.security.auth}
                  onChange={(e) =>
                    handleSecuritySettingChange('auth', e.target.checked)
                  }
                  className='h-4 w-4 text-blue-600 border-gray-300 rounded focus:ring-blue-500'
                />
                <label className='ml-2 block text-sm text-gray-700 dark:text-gray-300'>
                  启用身份认证
                </label>
              </div>

              <div>
                <label className='block text-sm font-medium text-gray-700 mb-2'>
                  允许的主机地址
                </label>
                <div className='space-y-2'>
                  {settings.security.allowed_hosts.map((host, index) => (
                    <div key={index} className='flex items-center gap-2'>
                      <input
                        type='text'
                        value={host}
                        onChange={(e) => {
                          const newHosts = [...settings.security.allowed_hosts]
                          newHosts[index] = e.target.value
                          handleSecuritySettingChange('allowed_hosts', newHosts)
                        }}
                        className='input-modern flex-1'
                      />
                      <button
                        onClick={() => removeAllowedHost(index)}
                        className='btn-modern bg-red-500 hover:bg-red-600 text-white px-3 py-1'>
                        删除
                      </button>
                    </div>
                  ))}
                  <button
                    onClick={addAllowedHost}
                    className='btn-modern bg-blue-500 hover:bg-blue-600 text-white px-4 py-2'>
                    ➕ 添加主机
                  </button>
                </div>
                <p className='text-xs text-gray-500 mt-1'>
                  允许访问MCP Router的主机地址列表
                </p>
              </div>
            </div>
          </div>
        )}

        {/* Tools Settings Tab */}
        {activeTab === 'tools' && (
          <div className='space-y-6'>
            <h3 className='text-xl font-semibold text-gray-800 dark:text-gray-100 mb-4'>
              🧰 工具设置
            </h3>
            <div className='space-y-6'>
              {/* Package Mirror Settings */}
              <div className='space-y-4'>
                <div>
                  <div className='flex items-center gap-4'>
                    <label className='text-sm font-medium text-gray-700 dark:text-gray-300 w-32 flex-shrink-0'>
                      🧪 Pypi 镜像 URL
                    </label>
                    <input
                      type='text'
                      value={settings.settings?.uv_index_url || ''}
                      onChange={(e) => {
                        setSettings((prev) => ({
                          ...prev,
                          settings: {
                            ...prev.settings,
                            uv_index_url: e.target.value,
                          },
                        }))
                      }}
                      defaultValue={settings.settings?.uv_index_url || ''}
                      className='input-modern flex-1'
                      placeholder='例如：https://pypi.tuna.tsinghua.edu.cn/simple'
                    />
                  </div>
                  <p className='text-xs text-gray-500 mt-1 ml-36'>
                    设置环境变量 `UV_INDEX_URL`，影响 uv/uvx 包索引来源
                  </p>
                </div>

                <div>
                  <div className='flex items-center gap-4'>
                    <label className='text-sm font-medium text-gray-700 dark:text-gray-300 w-32 flex-shrink-0'>
                      📦 npm registry
                    </label>
                    <input
                      type='text'
                      value={settings.settings?.npm_registry || ''}
                      onChange={(e) => {
                        setSettings((prev) => ({
                          ...prev,
                          settings: {
                            ...prev.settings,
                            npm_registry: e.target.value,
                          },
                        }))
                      }}
                      className='input-modern flex-1'
                      placeholder='例如：https://registry.npmmirror.com'
                    />
                  </div>
                  <p className='text-xs text-gray-500 mt-1 ml-36'>
                    设置环境变量 `NPM_CONFIG_REGISTRY`，影响 npx/npm 包源
                  </p>
                </div>
              </div>

              <div className='bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-lg p-4'>
                <p className='text-xs text-amber-700'>
                  更改镜像后仅影响新启动的 STDIO
                  服务进程；已连接的服务需重启或重新连接。
                </p>
              </div>
            </div>
          </div>
        )}

        {/* Application Settings Tab */}
        {activeTab === 'application' && (
          <div className='space-y-6'>
            <div>
              <h3 className='text-xl font-semibold text-gray-800 dark:text-gray-100 mb-4'>
                ⚙️ 应用设置
              </h3>

              <div className='space-y-6'>
                {/* Autostart Setting */}
                <div>
                  <div className='flex items-center justify-between'>
                    <div>
                      <label className='text-sm font-medium text-gray-700 dark:text-gray-300'>
                        🚀 开机自动启动
                      </label>
                      <p className='text-xs text-gray-500 mt-1'>
                        启用后，MCP Router 将在系统启动时自动运行
                      </p>
                    </div>
                    <button
                      onClick={toggleAutostart}
                      className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 ${
                        autostartEnabled ? 'bg-blue-500' : 'bg-gray-300'
                      }`}
                      aria-label='Toggle autostart'>
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                          autostartEnabled ? 'translate-x-6' : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </div>
                </div>

                {/* System Tray Settings */}
                <div className='space-y-4'>
                  <div className='flex items-center justify-between'>
                    <div>
                      <label className='text-sm font-medium text-gray-700 dark:text-gray-300'>
                        📱 启用系统托盘
                      </label>
                      <p className='text-xs text-gray-500 mt-1'>
                        关闭主窗口后最小化到系统托盘，并通过托盘菜单访问常用功能
                      </p>
                    </div>
                    <button
                      onClick={toggleSystemTrayEnabled}
                      className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 ${
                        settings.settings?.system_tray?.enabled ?? true
                          ? 'bg-blue-500'
                          : 'bg-gray-300'
                      }`}
                      aria-label='Toggle system tray'>
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                          settings.settings?.system_tray?.enabled ?? true
                            ? 'translate-x-6'
                            : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </div>

                  {/* Minimize to tray on close */}
                  <div className='flex items-center justify-between'>
                    <div>
                      <label className='text-sm font-medium text-gray-700 dark:text-gray-300'>
                        🪟 关闭时最小化到托盘
                      </label>
                      <p className='text-xs text-gray-500 mt-1'>
                        启用后，点击窗口关闭按钮不会退出应用，只会隐藏到托盘
                      </p>
                    </div>
                    <button
                      onClick={toggleMinimizeOnClose}
                      disabled={
                        !(settings.settings?.system_tray?.enabled ?? true)
                      }
                      className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 ${
                        settings.settings?.system_tray?.close_to_tray ?? false
                          ? 'bg-blue-500'
                          : 'bg-gray-300'
                      } ${
                        !(settings.settings?.system_tray?.enabled ?? true)
                          ? 'opacity-50 cursor-not-allowed'
                          : ''
                      }`}
                      aria-label='Toggle minimize to tray on close'>
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                          settings.settings?.system_tray?.close_to_tray ?? false
                            ? 'translate-x-6'
                            : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </div>

                  {/* Minimize to tray on start */}
                  <div className='flex items-center justify-between'>
                    <div>
                      <label className='text-sm font-medium text-gray-700 dark:text-gray-300'>
                        🟨 启动时最小化到托盘
                      </label>
                      <p className='text-xs text-gray-500 mt-1'>
                        启用后，应用启动时将直接隐藏到托盘（不显示主窗口）
                      </p>
                    </div>
                    <button
                      onClick={toggleMinimizeOnStart}
                      disabled={
                        !(settings.settings?.system_tray?.enabled ?? true)
                      }
                      className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 ${
                        settings.settings?.system_tray?.start_to_tray ?? false
                          ? 'bg-blue-500'
                          : 'bg-gray-300'
                      } ${
                        !(settings.settings?.system_tray?.enabled ?? true)
                          ? 'opacity-50 cursor-not-allowed'
                          : ''
                      }`}
                      aria-label='Toggle minimize to tray on start'>
                      <span
                        className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                          settings.settings?.system_tray?.start_to_tray ?? false
                            ? 'translate-x-6'
                            : 'translate-x-1'
                        }`}
                      />
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Save Button */}
      <div className='flex justify-end'>
        {activeTab !== 'application' && (
          <button
            onClick={async () => {
              const ae = document.activeElement as HTMLElement | null
              ae?.blur()
              await saveSettings()
            }}
            disabled={saving}
            className='btn-modern btn-primary-modern px-8'>
            {saving ? '保存中...' : '💾 保存设置'}
          </button>
        )}
      </div>
    </div>
  )
}

export default Settings
