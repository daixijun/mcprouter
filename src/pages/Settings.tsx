import { invoke } from '@tauri-apps/api/core'
import {
  App,
  Button,
  Card,
  Col,
  Flex,
  Input,
  InputNumber,
  Row,
  Select,
  Switch,
  Typography,
} from 'antd'
import { memo, useCallback, useEffect, useState } from 'react'
import { useErrorContext } from '../contexts/ErrorContext'
import type { SystemSettings } from '../types'

const { Title, Text } = Typography

const Settings: React.FC = memo(() => {
  const { addError } = useErrorContext()
  const { message } = App.useApp()

  // State
  const [settings, setSettings] = useState<SystemSettings>({
    server: {
      host: 'localhost',
      port: 8850,
      max_connections: 100,
      timeout_seconds: 30,
      auth: false,
    },
    logging: {
      level: 'info',
      file_name: '',
    },
    settings: {
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
  const [autostartEnabled, setAutostartEnabled] = useState(false)
  const [localIPs, setLocalIPs] = useState<string[]>([])
  const [loadingIPs, setLoadingIPs] = useState(false)

  const loadLocalIPs = useCallback(async () => {
    setLoadingIPs(true)
    try {
      const ips = await invoke<string[]>('get_local_ip_addresses')
      setLocalIPs(ips)
    } catch (error) {
      console.error('Failed to load local IPs:', error)
      addError('获取本地IP地址失败')
    } finally {
      setLoadingIPs(false)
    }
  }, [addError])

  // Data fetching
  useEffect(() => {
    const loadData = async () => {
      try {
        await Promise.all([loadSettings(), loadLocalIPs()])
      } catch (error) {
        console.error('Failed to load initial data:', error)
      }
    }

    loadData()
  }, [])

  const loadSettings = useCallback(async () => {
    setLoading(true)
    try {
      const { ConfigService } = await import('../services/config-service')
      // 添加超时处理，防止请求卡住
      const timeoutPromise = new Promise<SystemSettings>((_, reject) => {
        setTimeout(() => reject(new Error('加载设置超时')), 10000)
      })

      const loadedSettings = await Promise.race([
        ConfigService.getSystemSettings(),
        timeoutPromise,
      ])

      // 同时更新设置和自启动状态，避免重复调用接口
      setSettings(loadedSettings)
      setAutostartEnabled(loadedSettings.settings?.autostart || false)
    } catch (error) {
      console.error('Failed to load settings:', error)
      const errorMessage =
        error instanceof Error ? error.message : '加载系统设置失败'
      addError(errorMessage)
    } finally {
      setLoading(false)
    }
  }, [addError])

  const saveSettings = useCallback(async () => {
    console.log('=== SAVE SETTINGS START ===')
    console.log('Current settings state:', JSON.stringify(settings, null, 2))

    setSaving(true)

    try {
      console.log('1. Importing ConfigService...')
      const { ConfigService } = await import('../services/config-service')
      console.log(
        '2. ConfigService imported successfully:',
        typeof ConfigService,
      )

      console.log(
        '3. About to save settings:',
        JSON.stringify(settings, null, 2),
      )
      console.log('4. Calling ConfigService.saveSystemSettings...')

      const result = await ConfigService.saveSystemSettings(settings)
      console.log('5. Save successful, result:', result)

      // 不再重新加载设置，避免页面刷新
      // 状态已经通过 handleSystemTraySettingChange 的即时更新保持同步

      message.success('设置保存成功！系统配置已更新。')
      console.log('=== SAVE SETTINGS SUCCESS ===')
    } catch (error) {
      console.error('=== SAVE SETTINGS ERROR ===')
      console.error('Error details:', error)
      console.error('Error name:', (error as Error)?.name || 'Unknown')
      console.error(
        'Error message:',
        (error as Error)?.message || String(error),
      )
      console.error('Error stack:', (error as Error)?.stack || 'No stack trace')
      addError('保存设置失败')
    } finally {
      console.log('=== SAVE SETTINGS FINALLY ===')
      setSaving(false)
    }
  }, [settings, addError])

  // Handler functions
  const handleServerSettingChange = useCallback((key: string, value: any) => {
    setSettings((prev) => ({
      ...prev,
      server: {
        ...prev.server,
        [key]: value,
      },
    }))
  }, [])

  const handleLoggingSettingChange = useCallback((key: string, value: any) => {
    setSettings((prev) => ({
      ...prev,
      logging: {
        ...prev.logging,
        [key]: value,
      },
    }))
  }, [])

  // security removed

  const handleSystemTraySettingChange = useCallback(
    async (key: string, value: any) => {
      // 立即更新本地状态
      const newSettings = {
        ...settings,
        settings: {
          ...settings.settings,
          system_tray: {
            ...settings.settings?.system_tray,
            [key]: value,
          },
        },
      }
      setSettings(newSettings)

      // 对于系统托盘启用状态，立即保存并应用变更
      if (key === 'enabled') {
        try {
          console.log('=== IMMEDIATE TRAY SETTINGS UPDATE ===')
          console.log(`Updating system_tray.${key} to:`, value)

          const { ConfigService } = await import('../services/config-service')
          await ConfigService.saveSystemSettings(newSettings)

          console.log('System tray settings updated immediately')

          // 显示成功提示
          message.success('系统托盘设置已更新')

          // 不重新加载设置，因为状态已经更新，避免可能的页面刷新
          // 如果后端返回的数据有差异，可以在响应后更新特定字段

          console.log('=== IMMEDIATE TRAY SETTINGS SUCCESS ===')
        } catch (error) {
          console.error('Failed to update system tray settings:', error)
          addError('更新系统托盘设置失败')

          // 如果保存失败，恢复原来的状态
          setSettings(settings)
        }
      }
    },
    [settings, addError],
  )

  const toggleAutostart = useCallback(async () => {
    try {
      const { ConfigService } = await import('../services/config-service')
      const result = await ConfigService.toggleAutostart()
      const newState = !autostartEnabled
      setAutostartEnabled(newState)
      message.success(result)
    } catch (error) {
      console.error('Failed to toggle autostart:', error)
      addError('切换开机自启失败')
    }
  }, [autostartEnabled, addError])

  // security removed

  if (loading) {
    return (
      <Flex justify='center' align='center' style={{ height: '256px' }}>
        <Button loading>加载设置...</Button>
      </Flex>
    )
  }

  return (
    <Flex
      vertical
      gap='large'
      style={{ height: '100%', overflowY: 'auto', padding: '24px' }}>
      {/* Settings Content */}
      <Flex vertical gap='large' style={{ flex: 1 }}>
        {/* Server Settings */}
        <Card>
          <Title level={4}>服务器配置</Title>
          <Row gutter={[16, 16]}>
            <Col xs={24} md={12}>
              <Text strong>服务器地址</Text>
              <Select
                value={settings.server.host}
                onChange={(value) => handleServerSettingChange('host', value)}
                loading={loadingIPs}
                style={{ width: '100%', marginTop: '4px' }}
                placeholder='选择服务器地址'
                options={localIPs.map((ip) => ({ value: ip, label: ip }))}
              />
            </Col>
            <Col xs={24} md={12}>
              <Text strong>端口</Text>
              <InputNumber
                value={settings.server.port}
                onChange={(value) =>
                  handleServerSettingChange('port', value || 0)
                }
                min={1}
                max={65535}
                style={{ width: '100%', marginTop: '4px' }}
              />
            </Col>
            <Col xs={24} md={12}>
              <Text strong>最大连接数</Text>
              <InputNumber
                value={settings.server.max_connections}
                onChange={(value) =>
                  handleServerSettingChange('max_connections', value || 0)
                }
                min={1}
                max={1000}
                style={{ width: '100%', marginTop: '4px' }}
              />
            </Col>
            <Col xs={24} md={12}>
              <Text strong>超时时间（秒）</Text>
              <InputNumber
                value={settings.server.timeout_seconds}
                onChange={(value) =>
                  handleServerSettingChange('timeout_seconds', value || 0)
                }
                min={1}
                max={300}
                style={{ width: '100%', marginTop: '4px' }}
              />
            </Col>
            <Col xs={24}>
              <Flex justify='space-between' align='center'>
                <div>
                  <Text strong>聚合接口认证鉴权</Text>
                  <Text
                    type='secondary'
                    style={{
                      fontSize: '14px',
                      display: 'block',
                      marginTop: '2px',
                    }}>
                    启用后，聚合接口需要使用有效的Bearer Token才能访问
                  </Text>
                </div>
                <Switch
                  checked={settings.server.auth || false}
                  onChange={async (checked) => {
                    handleServerSettingChange('auth', checked)
                    // 立即保存设置
                    try {
                      const { ConfigService } = await import('../services/config-service')
                      await ConfigService.saveSystemSettings(settings)
                      message.success('认证设置已保存！')
                    } catch (error) {
                      console.error('Failed to save auth setting:', error)
                      message.error('保存认证设置失败')
                    }
                  }}
                  checkedChildren='启用'
                  unCheckedChildren='禁用'
                />
              </Flex>
            </Col>
          </Row>
        </Card>

        {/* Logging Settings */}
        <Card>
          <Title level={4}>日志配置</Title>
          <Row gutter={[16, 16]}>
            <Col xs={24} md={12}>
              <Text strong>日志级别</Text>
              <Select
                style={{ width: '100%', marginTop: '4px' }}
                value={settings.logging.level}
                onChange={(value) => handleLoggingSettingChange('level', value)}
                options={[
                  { value: 'trace', label: 'Trace' },
                  { value: 'debug', label: 'Debug' },
                  { value: 'info', label: 'Info' },
                  { value: 'warn', label: 'Warning' },
                  { value: 'error', label: 'Error' },
                ]}
              />
            </Col>
            <Col xs={24} md={12}>
              <Text strong>日志文件名</Text>
              <Input
                value={settings.logging.file_name}
                onChange={(e) =>
                  handleLoggingSettingChange('file_name', e.target.value)
                }
                placeholder='mcp-router.log'
                style={{ marginTop: '4px' }}
              />
            </Col>
          </Row>
        </Card>

        {/* Application Settings */}
        <Card>
          <Title level={4}>应用配置</Title>
          <Flex vertical gap='large'>
            <div>
              <Title level={5}>系统托盘</Title>
              <Flex vertical gap='middle'>
                <Flex justify='space-between' align='center'>
                  <div>
                    <Text strong>启用系统托盘</Text>
                    <Text
                      type='secondary'
                      style={{
                        fontSize: '14px',
                        display: 'block',
                        marginTop: '2px',
                      }}>
                      在系统托盘显示应用图标
                    </Text>
                  </div>
                  <Switch
                    checked={settings.settings?.system_tray?.enabled}
                    onChange={(checked) =>
                      handleSystemTraySettingChange('enabled', checked)
                    }
                    checkedChildren='启用'
                    unCheckedChildren='禁用'
                  />
                </Flex>

                <Flex justify='space-between' align='center'>
                  <div>
                    <Text
                      strong={!settings.settings?.system_tray?.enabled}
                      type={
                        !settings.settings?.system_tray?.enabled
                          ? 'secondary'
                          : undefined
                      }>
                      关闭到托盘
                    </Text>
                    <Text
                      type='secondary'
                      style={{
                        fontSize: '14px',
                        display: 'block',
                        marginTop: '2px',
                      }}>
                      关闭窗口时最小化到托盘
                    </Text>
                  </div>
                  <Switch
                    checked={settings.settings?.system_tray?.close_to_tray}
                    onChange={(checked) =>
                      handleSystemTraySettingChange('close_to_tray', checked)
                    }
                    checkedChildren='启用'
                    unCheckedChildren='禁用'
                    disabled={!settings.settings?.system_tray?.enabled}
                  />
                </Flex>

                <Flex justify='space-between' align='center'>
                  <div>
                    <Text strong>开机自启</Text>
                    <Text
                      type='secondary'
                      style={{
                        fontSize: '14px',
                        display: 'block',
                        marginTop: '2px',
                      }}>
                      系统启动时自动运行应用
                    </Text>
                  </div>
                  <Switch
                    checked={autostartEnabled}
                    onChange={toggleAutostart}
                    checkedChildren='启用'
                    unCheckedChildren='禁用'
                  />
                </Flex>
              </Flex>
            </div>
          </Flex>
        </Card>
      </Flex>

      {/* Bottom Save Button */}
      <div
        style={{
          marginTop: 'auto',
          paddingTop: '20px',
          borderTop: '1px solid var(--ant-color-border)',
          display: 'flex',
          justifyContent: 'flex-end',
        }}>
        <Button
          onClick={() => {
            console.log('=== SAVE BUTTON CLICKED ===')
            console.log('Button onClick triggered, calling saveSettings...')
            saveSettings()
          }}
          loading={saving}
          type='primary'
          size='large'
          style={{ minWidth: '120px' }}>
          {saving ? '保存中...' : '保存设置'}
        </Button>
      </div>
    </Flex>
  )
})

export default Settings
