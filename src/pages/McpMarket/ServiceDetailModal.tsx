import { Modal, Tag, Empty, Button, Space, Table } from 'antd'
import { useTranslation } from 'react-i18next'
import type { MarketServiceDetail } from '../../types/mcp-market'

interface ServiceDetailModalProps {
  visible: boolean
  service: MarketServiceDetail | null
  onClose: () => void
}

export default function ServiceDetailModal({
  visible,
  service,
  onClose,
}: ServiceDetailModalProps) {
  const { t } = useTranslation()

  if (!service) {
    return null
  }

  return (
    <Modal
      title={
        <div className="flex items-center gap-2">
          <span className="text-lg font-semibold">{service.displayName}</span>
          <Tag color="blue" className="text-xs">
            {t('market.service.version')}
          </Tag>
        </div>
      }
      open={visible}
      onCancel={onClose}
      footer={
        <Button type="primary" onClick={onClose}>
          {t('common.close')}
        </Button>
      }
      width={800}
    >
      <Space direction="vertical" size="large" className="w-full">
        {/* Readme */}
        {service.readme && (
          <div className="mb-6">
            <h3 className="text-lg font-semibold mb-3 text-gray-900 dark:text-gray-100">
              {t('market.detail.readme')}
            </h3>
            <div className="bg-gray-50 dark:bg-gray-800 p-4 rounded-lg overflow-auto max-h-96">
              <pre className="whitespace-pre-wrap text-sm">{service.readme}</pre>
            </div>
          </div>
        )}

        {/* Environment variables */}
        {service.envSchema && service.envSchema.length > 0 && (
          <div className="mb-6">
            <h3 className="text-lg font-semibold mb-3 text-gray-900 dark:text-gray-100">
              {t('market.detail.env_vars')}
            </h3>
            <Table
              dataSource={service.envSchema.map((env, index) => ({
                key: index,
                name: env.name,
                label: env.label,
                type: env.type,
                required: env.required,
                default: env.default,
                description: env.description,
              }))}
              columns={[
                {
                  title: t('market.detail.env_name'),
                  dataIndex: 'name',
                  key: 'name',
                  width: 150,
                },
                {
                  title: t('market.detail.env_type'),
                  dataIndex: 'type',
                  key: 'type',
                  width: 100,
                  render: (type: string) => (
                    <Tag color={type === 'secret' ? 'red' : 'blue'}>
                      {type}
                    </Tag>
                  ),
                },
                {
                  title: t('market.detail.env_required'),
                  dataIndex: 'required',
                  key: 'required',
                  width: 80,
                  render: (required: boolean) => (
                    required ? (
                      <span className="text-green-600">✓</span>
                    ) : (
                      <span className="text-gray-400">-</span>
                    )
                  ),
                },
                {
                  title: t('market.detail.env_default'),
                  dataIndex: 'default',
                  key: 'default',
                  width: 120,
                  render: (value) => value || '-',
                },
                {
                  title: t('market.detail.env_description'),
                  dataIndex: 'description',
                  key: 'description',
                },
              ]}
              pagination={false}
              size="small"
            />
          </div>
        )}

        {/* Transport types */}
        {service.transportTypes && service.transportTypes.length > 0 && (
          <div className="mb-6">
            <h3 className="text-lg font-semibold mb-3 text-gray-900 dark:text-gray-100">
              {t('market.detail.transports')}
            </h3>
            <Space wrap>
              {service.transportTypes.map((type) => (
                <Tag key={type} color="green" className="text-sm">
                  {type.toUpperCase()}
                </Tag>
              ))}
            </Space>
          </div>
        )}

        {/* No details */}
        {!service.readme && !service.envSchema && (
          <Empty description={t('market.detail.no_details')} />
        )}
      </Space>
    </Modal>
  )
}
