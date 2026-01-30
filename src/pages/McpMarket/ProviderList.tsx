import type { MarketProvider } from '../../types/mcp-market'
import { useTranslation } from 'react-i18next'

interface ProviderListProps {
  providers: MarketProvider[]
  selectedProviderId: string | null
  onProviderSelect: (providerId: string) => void
}

export default function ProviderList({
  providers,
  selectedProviderId,
  onProviderSelect,
}: ProviderListProps) {
  const { i18n } = useTranslation()

  // 获取本地化的 Provider 名称
  const getProviderName = (provider: MarketProvider): string => {
    const isZh = i18n.language.startsWith('zh')

    if (provider.id() === 'modelscope') {
      return isZh ? '魔搭社区' : 'ModelScope'
    }

    return provider.name()
  }

  return (
    <div>
      {providers.map((provider) => {
        const isSelected = provider.id() === selectedProviderId
        return (
          <div
            key={provider.id()}
            onClick={() => onProviderSelect(provider.id())}
            className={`
              provider-item
              cursor-pointer
              rounded-md
              px-3
              py-2.5
              mb-2
              transition-all
              duration-200
              flex
              items-center
              gap-3
              ${isSelected
                ? 'bg-blue-50 dark:bg-blue-900/20 text-blue-600 dark:text-blue-400 font-medium'
                : 'text-gray-700 dark:text-gray-300 hover:bg-gray-100 dark:hover:bg-gray-800'
              }
            `}
            style={{
              position: 'relative',
            }}
          >
            {/* 左侧选中指示器 */}
            {isSelected && (
              <div
                className="absolute left-0 top-1/2 -translate-y-1/2 w-1 h-8 rounded-r"
                style={{
                  background: 'linear-gradient(to bottom, #3b82f6, #2563eb)',
                }}
              />
            )}

            {/* Provider Icon */}
            {provider.iconUrl() && (
              <img
                src={provider.iconUrl()}
                alt={provider.name()}
                className="w-5 h-5 flex-shrink-0"
                style={{
                  filter: isSelected ? 'none' : 'grayscale(0.3)',
                }}
              />
            )}

            {/* Provider Name */}
            <span className="truncate text-sm">{getProviderName(provider)}</span>
          </div>
        )
      })}
    </div>
  )
}
