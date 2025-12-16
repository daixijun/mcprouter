import { invoke } from '@tauri-apps/api/core'
import {
  App,
  Button,
  Col,
  Flex,
  Input,
  InputNumber,
  Row,
  Select,
  Space,
  Switch,
  Typography,
} from 'antd'
// 显式导入Card组件，解决类型问题
import Card from 'antd/es/card'
import { memo, useCallback, useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'
import SystemToolManager from '../components/SystemToolManager'
import type { SystemSettings } from '../types'

const { Title, Text } = Typography

const Settings: React.FC = memo(() => {
  const { t } = useTranslation()
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
      sql_log: false,
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
      message.error(t('settings.errors.load_local_ips_failed'))
    } finally {
      setLoadingIPs(false)
    }
  }, [message.error])

  const loadSettings = useCallback(async () => {
    setLoading(true)
    try {
      const { ConfigService } = await import('../services/config-service')
      // 添加超时处理，防止请求卡住
      const timeoutPromise = new Promise<SystemSettings>((_, reject) => {
        setTimeout(
          () => reject(new Error(t('settings.errors.load_settings_timeout'))),
          10000,
        )
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
        error instanceof Error
          ? error.message
          : t('settings.errors.load_system_settings_failed')
      message.error(errorMessage)
    } finally {
      setLoading(false)
    }
  }, [message.error])

  
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
  }, [loadSettings, loadLocalIPs])

  const saveSettings = useCallback(async () => {
    setSaving(true)

    try {
      const { ConfigService } = await import('../services/config-service')
      await ConfigService.saveSystemSettings(settings)

      // 不再重新加载设置，避免页面刷新
      // 状态已经通过 handleSystemTraySettingChange 的即时更新保持同步

      message.success(t('settings.messages.save_success'))
    } catch (error) {
      message.error(t('settings.errors.save_settings_failed'))
    } finally {
          setSaving(false)
    }
  }, [settings, message.error])

  // Handler functions
  const handleServerSettingChange = useCallback(
    (key: string, value: string | number | boolean) => {
      setSettings((prev) => ({
        ...prev,
        server: {
          ...prev.server,
          [key]: value,
        },
      }))
    },
    [],
  )

  const handleLoggingSettingChange = useCallback(
    (key: string, value: string | number | boolean) => {
      setSettings((prev) => ({
        ...prev,
        logging: {
          ...prev.logging,
          [key]: value,
        },
      }))
    },
    [],
  )

  // security removed

  const handleSystemTraySettingChange = useCallback(
    async (key: string, value: boolean) => {
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
  
          const { ConfigService } = await import('../services/config-service')
          await ConfigService.saveSystemSettings(newSettings)

        
          // 显示成功提示
          message.success(t('settings.messages.tray_settings_updated'))

          // 不重新加载设置，因为状态已经更新，避免可能的页面刷新
          // 如果后端返回的数据有差异，可以在响应后更新特定字段

                } catch (error) {
          console.error('Failed to update system tray settings:', error)
          message.error(t('settings.errors.update_tray_settings_failed'))

          // 如果保存失败，恢复原来的状态
          setSettings(settings)
        }
      }
    },
    [settings, message.error],
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
      message.error(t('settings.errors.toggle_autostart_failed'))
    }
  }, [autostartEnabled, message.error])

  
  // security removed

  if (loading) {
    return (
      <Flex justify='center' align='center' style={{ height: '256px' }}>
        <Button loading>{t('settings.loading')}</Button>
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
          <Title level={4}>{t('settings.server.title')}</Title>
          <Row gutter={[16, 16]}>
            <Col xs={24} md={12}>
              <Text strong>{t('settings.server.host')}</Text>
              <Select
                value={settings.server.host}
                onChange={(value: string) =>
                  handleServerSettingChange('host', value)
                }
                loading={loadingIPs}
                style={{ width: '100%', marginTop: '4px' }}
                placeholder={t('settings.server.select_host')}
                options={localIPs.map((ip) => ({ value: ip, label: ip }))}
              />
            </Col>
            <Col xs={24} md={12}>
              <Text strong>{t('settings.server.port')}</Text>
              <InputNumber
                value={settings.server.port}
                onChange={(value: number | null) =>
                  handleServerSettingChange('port', value || 0)
                }
                min={1}
                max={65535}
                style={{ width: '100%', marginTop: '4px' }}
              />
            </Col>
            <Col xs={24} md={12}>
              <Text strong>{t('settings.server.max_connections')}</Text>
              <InputNumber
                value={settings.server.max_connections}
                onChange={(value: number | null) =>
                  handleServerSettingChange('max_connections', value || 0)
                }
                min={1}
                max={1000}
                style={{ width: '100%', marginTop: '4px' }}
              />
            </Col>
            <Col xs={24} md={12}>
              <Text strong>{t('settings.server.timeout')}</Text>
              <InputNumber
                value={settings.server.timeout_seconds}
                onChange={(value: number | null) =>
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
                  <Text strong>{t('settings.server.auth.title')}</Text>
                  <Text
                    type='secondary'
                    style={{
                      fontSize: '14px',
                      display: 'block',
                      marginTop: '2px',
                    }}>
                    {t('settings.server.auth.description')}
                  </Text>
                </div>
                <Switch
                  checked={settings.server.auth || false}
                  onChange={async (checked: boolean) => {
                    handleServerSettingChange('auth', checked)
                    // 立即保存设置
                    try {
                      const { ConfigService } = await import(
                        '../services/config-service'
                      )
                      await ConfigService.saveSystemSettings(settings)
                      message.success(
                        t('settings.messages.auth_settings_saved'),
                      )
                    } catch (error) {
                      console.error('Failed to save auth setting:', error)
                      message.error(
                        t('settings.errors.save_auth_settings_failed'),
                      )
                    }
                  }}
                />
              </Flex>
            </Col>
          </Row>
        </Card>

        {/* Logging Settings */}
        <Card>
          <Title level={4}>{t('settings.logging.title')}</Title>
          <Row gutter={[16, 16]}>
            <Col xs={24} md={12}>
              <Text strong>{t('settings.logging.level')}</Text>
              <Select
                style={{ width: '100%', marginTop: '4px' }}
                value={settings.logging.level}
                onChange={(value: string) =>
                  handleLoggingSettingChange('level', value)
                }
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
              <Text strong>{t('settings.logging.file_name')}</Text>
              <Input
                value={settings.logging.file_name}
                onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                  handleLoggingSettingChange('file_name', e.target.value)
                }
                placeholder='mcp-router.log'
                style={{ marginTop: '4px' }}
              />
            </Col>
          </Row>
          <Row gutter={[16, 16]} style={{ marginTop: '16px' }}>
            <Col xs={24} md={12}>
              <Space>
                <Switch
                  checked={settings.logging.sql_log}
                  onChange={(checked: boolean) =>
                    handleLoggingSettingChange('sql_log', checked)
                  }
                />
                <Text strong>{t('settings.logging.sql_log')}</Text>
              </Space>
              <div style={{ marginTop: '4px' }}>
                <Text type="secondary" style={{ fontSize: '12px' }}>
                  {t('settings.logging.sql_log_description')}
                </Text>
              </div>
            </Col>
          </Row>
        </Card>

        {/* Application Settings */}
        <Card>
          <Title level={4}>{t('settings.app.title')}</Title>
          <Flex vertical gap='large'>
            <div>
              <Title level={5}>{t('settings.app.system_tray.title')}</Title>
              <Flex vertical gap='middle'>
                <Flex justify='space-between' align='center'>
                  <div>
                    <Text strong>{t('settings.app.system_tray.enable')}</Text>
                    <Text
                      type='secondary'
                      style={{
                        fontSize: '14px',
                        display: 'block',
                        marginTop: '2px',
                      }}>
                      {t('settings.app.system_tray.enable_description')}
                    </Text>
                  </div>
                  <Switch
                    checked={settings.settings?.system_tray?.enabled}
                    onChange={(checked: boolean) =>
                      handleSystemTraySettingChange('enabled', checked)
                    }
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
                      {t('settings.app.system_tray.close_to_tray')}
                    </Text>
                    <Text
                      type='secondary'
                      style={{
                        fontSize: '14px',
                        display: 'block',
                        marginTop: '2px',
                      }}>
                      {t('settings.app.system_tray.close_to_tray_description')}
                    </Text>
                  </div>
                  <Switch
                    checked={settings.settings?.system_tray?.close_to_tray}
                    onChange={(checked: boolean) =>
                      handleSystemTraySettingChange('close_to_tray', checked)
                    }
                    disabled={!settings.settings?.system_tray?.enabled}
                  />
                </Flex>

                <Flex justify='space-between' align='center'>
                  <div>
                    <Text strong>{t('settings.app.autostart.title')}</Text>
                    <Text
                      type='secondary'
                      style={{
                        fontSize: '14px',
                        display: 'block',
                        marginTop: '2px',
                      }}>
                      {t('settings.app.autostart.description')}
                    </Text>
                  </div>
                  <Switch
                    checked={autostartEnabled}
                    onChange={toggleAutostart}
                  />
                </Flex>
              </Flex>
            </div>
          </Flex>
        </Card>
        </Flex>

        {/* Tool Management */}
        <Card>
          <SystemToolManager />
        </Card>

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
            saveSettings()
          }}
          loading={saving}
          type='primary'
          size='large'
          style={{ minWidth: '120px' }}>
          {saving ? t('settings.common.saving') : t('settings.actions.save')}
        </Button>
      </div>
    </Flex>
  )
})

export default Settings
