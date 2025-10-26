import React, { useEffect, useState } from 'react'
import { ApiService } from '../services/api'
import toastService from '../services/toastService'

interface CacheStats {
  hits: number
  misses: number
  writes: number
  deletes: number
  errors: number
  total_operations: number
  avg_read_time_ms: number
  avg_write_time_ms: number
  cache_size_bytes: number
  entry_count: number
}

const CacheManager: React.FC = () => {
  const [cacheStats, setCacheStats] = useState<CacheStats | null>(null)
  const [cachedServices, setCachedServices] = useState<string[]>([])
  const [loading, setLoading] = useState(false)
  const [refreshingService, setRefreshingService] = useState<string | null>(null)

  const loadCacheStats = async () => {
    try {
      setLoading(true)
      const stats = await ApiService.getCacheStats()
      setCacheStats(stats)
      setCachedServices([])
    } catch (error) {
      console.error('Failed to load cache stats:', error)
      toastService.sendErrorNotification('加载缓存统计失败')
    } finally {
      setLoading(false)
    }
  }

  const handleClearAllCache = async () => {
    try {
      setLoading(true)
      await ApiService.clearAllCache()
      toastService.sendSuccessNotification('缓存已清空')
      await loadCacheStats()
    } catch (error) {
      console.error('Failed to clear cache:', error)
      toastService.sendErrorNotification('清空缓存失败')
    } finally {
      setLoading(false)
    }
  }

  const handleFlushCache = async () => {
    try {
      setLoading(true)
      await ApiService.flushCache()
      toastService.sendSuccessNotification('缓存已刷新到磁盘')
      await loadCacheStats()
    } catch (error) {
      console.error('Failed to flush cache:', error)
      toastService.sendErrorNotification('刷新缓存失败')
    } finally {
      setLoading(false)
    }
  }

  const handleRefreshServiceCache = async (serviceName: string) => {
    try {
      setRefreshingService(serviceName)
      await ApiService.refreshServiceCache(serviceName)
      toastService.sendSuccessNotification(`${serviceName} 缓存已刷新`)
      await loadCacheStats()
    } catch (error) {
      console.error(`Failed to refresh cache for ${serviceName}:`, error)
      toastService.sendErrorNotification(`刷新 ${serviceName} 缓存失败`)
    } finally {
      setRefreshingService(null)
    }
  }

  const formatBytes = (bytes: number): string => {
    if (bytes === 0) return '0 B'
    const k = 1024
    const sizes = ['B', 'KB', 'MB', 'GB']
    const i = Math.floor(Math.log(bytes) / Math.log(k))
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
  }

  const formatTime = (ms: number): string => {
    if (ms < 1) return `${(ms * 1000).toFixed(1)}μs`
    if (ms < 1000) return `${ms.toFixed(1)}ms`
    return `${(ms / 1000).toFixed(1)}s`
  }

  const calculateHitRate = (): number => {
    if (!cacheStats || cacheStats.hits + cacheStats.misses === 0) return 0
    return (cacheStats.hits / (cacheStats.hits + cacheStats.misses)) * 100
  }

  useEffect(() => {
    loadCacheStats()
  }, [])

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex justify-between items-center">
        <div>
          <h2 className="text-2xl font-bold text-gray-800 dark:text-gray-100">
            🗄️ 缓存管理
          </h2>
          <p className="text-sm text-gray-600 dark:text-gray-300 mt-1">
            管理 MCP 服务缓存，提升响应性能
          </p>
        </div>
        <button
          onClick={loadCacheStats}
          disabled={loading}
          className="btn-modern btn-primary-modern">
          {loading ? '🔄 刷新中...' : '🔄 刷新统计'}
        </button>
      </div>

      {/* Cache Statistics */}
      {cacheStats && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          <div className="card-glass p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-gray-600 dark:text-gray-300">命中率</p>
                <p className="text-2xl font-bold text-green-600 dark:text-green-400">
                  {calculateHitRate().toFixed(1)}%
                </p>
              </div>
              <div className="text-2xl">🎯</div>
            </div>
          </div>

          <div className="card-glass p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-gray-600 dark:text-gray-300">缓存大小</p>
                <p className="text-2xl font-bold text-blue-600 dark:text-blue-400">
                  {formatBytes(cacheStats.cache_size_bytes)}
                </p>
              </div>
              <div className="text-2xl">💾</div>
            </div>
          </div>

          <div className="card-glass p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-gray-600 dark:text-gray-300">条目数量</p>
                <p className="text-2xl font-bold text-purple-600 dark:text-purple-400">
                  {cacheStats.entry_count.toLocaleString()}
                </p>
              </div>
              <div className="text-2xl">📊</div>
            </div>
          </div>

          <div className="card-glass p-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="text-sm text-gray-600 dark:text-gray-300">总操作数</p>
                <p className="text-2xl font-bold text-orange-600 dark:text-orange-400">
                  {cacheStats.total_operations.toLocaleString()}
                </p>
              </div>
              <div className="text-2xl">⚡</div>
            </div>
          </div>
        </div>
      )}

      {/* Detailed Statistics */}
      {cacheStats && (
        <div className="card-glass p-6">
          <h3 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4">
            📈 详细统计
          </h3>
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
            <div className="space-y-2">
              <p className="text-sm text-gray-600 dark:text-gray-300">缓存命中</p>
              <p className="text-lg font-semibold text-green-600 dark:text-green-400">
                {cacheStats.hits.toLocaleString()}
              </p>
            </div>
            <div className="space-y-2">
              <p className="text-sm text-gray-600 dark:text-gray-300">缓存未命中</p>
              <p className="text-lg font-semibold text-red-600 dark:text-red-400">
                {cacheStats.misses.toLocaleString()}
              </p>
            </div>
            <div className="space-y-2">
              <p className="text-sm text-gray-600 dark:text-gray-300">写入次数</p>
              <p className="text-lg font-semibold text-blue-600 dark:text-blue-400">
                {cacheStats.writes.toLocaleString()}
              </p>
            </div>
            <div className="space-y-2">
              <p className="text-sm text-gray-600 dark:text-gray-300">删除次数</p>
              <p className="text-lg font-semibold text-yellow-600 dark:text-yellow-400">
                {cacheStats.deletes.toLocaleString()}
              </p>
            </div>
            <div className="space-y-2">
              <p className="text-sm text-gray-600 dark:text-gray-300">错误次数</p>
              <p className="text-lg font-semibold text-red-600 dark:text-red-400">
                {cacheStats.errors.toLocaleString()}
              </p>
            </div>
            <div className="space-y-2">
              <p className="text-sm text-gray-600 dark:text-gray-300">平均读取时间</p>
              <p className="text-lg font-semibold text-purple-600 dark:text-purple-400">
                {formatTime(cacheStats.avg_read_time_ms)}
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Cache Actions */}
      <div className="card-glass p-6">
        <h3 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4">
          🛠️ 缓存操作
        </h3>
        <div className="flex flex-wrap gap-3">
          <button
            onClick={handleFlushCache}
            disabled={loading}
            className="btn-modern bg-blue-500 hover:bg-blue-600 text-white">
            💾 刷新到磁盘
          </button>
          <button
            onClick={handleClearAllCache}
            disabled={loading}
            className="btn-modern bg-red-500 hover:bg-red-600 text-white">
            🗑️ 清空所有缓存
          </button>
        </div>
      </div>

      {/* Cached Services */}
      {cachedServices.length > 0 && (
        <div className="card-glass p-6">
          <h3 className="text-lg font-semibold text-gray-800 dark:text-gray-100 mb-4">
            📋 已缓存的服务
          </h3>
          <div className="space-y-2">
            {cachedServices.map((serviceName) => (
              <div
                key={serviceName}
                className="flex items-center justify-between p-3 bg-gray-50 dark:bg-gray-700 rounded-lg">
                <div className="flex items-center space-x-3">
                  <div className="text-lg">🔧</div>
                  <span className="font-medium text-gray-800 dark:text-gray-100">
                    {serviceName}
                  </span>
                </div>
                <button
                  onClick={() => handleRefreshServiceCache(serviceName)}
                  disabled={refreshingService === serviceName}
                  className="btn-modern bg-green-500 hover:bg-green-600 text-white text-sm px-3 py-1">
                  {refreshingService === serviceName ? '🔄 刷新中...' : '🔄 刷新缓存'}
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Empty State */}
      {!loading && cachedServices.length === 0 && (
        <div className="card-glass p-8 text-center">
          <div className="text-4xl mb-3">📭</div>
          <h3 className="text-lg font-semibold text-gray-700 dark:text-gray-200 mb-2">
            暂无缓存数据
          </h3>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            当服务开始运行时，缓存数据将会显示在这里
          </p>
        </div>
      )}
    </div>
  )
}

export default CacheManager