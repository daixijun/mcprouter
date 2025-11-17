import { Button, Modal, Typography } from 'antd'
import React from 'react'
import { useTranslation } from 'react-i18next'

const { Title, Paragraph, Text } = Typography

interface AboutModalProps {
  isOpen: boolean
  onClose: () => void
}

const AboutModal: React.FC<AboutModalProps> = ({ isOpen, onClose }) => {
  const { t } = useTranslation()

  return (
    <Modal
      open={isOpen}
      onCancel={onClose}
      title={t('about.title')}
      footer={[
        <Button key='ok' type='primary' onClick={onClose}>
          {t('about.actions.ok')}
        </Button>,
      ]}
      width={600}>
      <div className='space-y-6'>
        {/* 头部描述 */}
        <Paragraph className='mb-6'>
          {t('about.description')}
        </Paragraph>

        {/* 功能特性 */}
        <div className='rounded-lg p-4'>
          <Title level={4} className='!mb-3'>
            {t('about.features.title')}
          </Title>
          <ul className='list-disc list-inside space-y-1 ml-2'>
            <li>{t('about.features.server_management')}</li>
            <li>{t('about.features.marketplace')}</li>
            <li>{t('about.features.system_tray')}</li>
            <li>{t('about.features.autostart')}</li>
          </ul>
        </div>

        {/* 版本与版权 */}
        <div className='rounded-lg p-4'>
          <Title level={4} className='!mb-3'>
            {t('about.version_info.title')}
          </Title>
          <div className='space-y-1'>
            <Text className='block'>{t('about.version_info.version')}</Text>
            <Text className='block'>
              {t('about.version_info.copyright')}
            </Text>
          </div>
        </div>
      </div>
    </Modal>
  )
}

export default AboutModal
