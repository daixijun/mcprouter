import type {
  MarketProvider,
  MarketService,
  MarketServiceDetail,
  ServiceSearchParams,
  ServiceSearchResult,
  ModelScopeMcpServer,
  ModelScopeMcpServerDetail,
  EnvVarSchema,
  TransportType,
} from '../types/mcp-market'

const BASE_URL = 'https://modelscope.cn'

export class ModelScopeProvider implements MarketProvider {
  id(): string {
    return 'modelscope'
  }

  name(): string {
    return '魔搭社区'
  }

  iconUrl(): string | undefined {
    return 'https://modelscope.cn/favicon.ico'
  }

  description(): string | undefined {
    return 'ModelScope - 阿里巴巴开源模型社区'
  }

  async searchServices(params: ServiceSearchParams, locale?: string): Promise<ServiceSearchResult> {
    const requestBody = {
      search: params.query || '',
      page_number: params.page,
      page_size: params.pageSize,
      filter: {
        category: params.category,
      },
    }

    const response = await fetch(`${BASE_URL}/openapi/v1/mcp/servers`, {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(requestBody),
    })

    if (!response.ok) {
      throw new Error(`Failed to fetch services: ${response.status} ${response.statusText}`)
    }

    const data = await response.json()

    if (!data.success) {
      throw new Error(data.message || 'Failed to fetch services from ModelScope')
    }

    // API 返回的字段名是 mcp_server_list（不是 mcp_servers）
    const servers: ModelScopeMcpServer[] = data.data.mcp_server_list || []
    const totalCount: number = data.data.total_count || 0
    const page: number = data.data.page_number || params.page
    const pageSize: number = data.data.page_size || params.pageSize

    // 使用传入的 locale 参数，如果没有则从 localStorage 读取
    const currentLang = locale || localStorage.getItem('i18nextLng') || 'zh-CN'

    const services = servers.map((server) => this.mapToMarketService(server, currentLang))

    return {
      services,
      totalCount,
      page,
      pageSize,
      hasMore: page * pageSize < totalCount,
    }
  }

  async getServiceDetail(serviceId: string, locale?: string): Promise<MarketServiceDetail> {
    const response = await fetch(`${BASE_URL}/openapi/v1/mcp/servers/${serviceId}`, {
      method: 'GET',
      headers: {
        'Content-Type': 'application/json',
      },
    })

    if (!response.ok) {
      throw new Error(`Failed to fetch service detail: ${response.status} ${response.statusText}`)
    }

    const data = await response.json()

    if (!data.success) {
      throw new Error(data.message || 'Failed to fetch service detail from ModelScope')
    }

    const server: ModelScopeMcpServerDetail = data.data

    // 使用传入的 locale 参数，如果没有则从 localStorage 读取
    const currentLang = locale || localStorage.getItem('i18nextLng') || 'zh-CN'

    return this.mapToMarketServiceDetail(server, currentLang)
  }

  private mapToMarketService(server: ModelScopeMcpServer, locale: string = 'zh-CN'): MarketService {
    // 根据当前语言获取本地化内容
    const getLocalizedString = () => {
      // 判断当前语言：zh-CN 使用中文，其他使用英文
      const isZh = locale.startsWith('zh')

      // 优先使用当前语言的 locale
      if (isZh && server.locales?.zh) {
        return server.locales.zh
      }
      if (!isZh && server.locales?.en) {
        return server.locales.en
      }

      // 如果当前语言不存在，尝试使用另一个语言
      if (server.locales?.zh) {
        return server.locales.zh
      }
      if (server.locales?.en) {
        return server.locales.en
      }

      // 如果都没有，返回 undefined
      return undefined
    }

    const localized = getLocalizedString()

    // 获取显示名称：优先使用本地化 name，然后是 chinese_name，最后是 name
    const displayName =
      localized?.name ||
      server.chinese_name ||
      server.name ||
      server.id

    // 获取描述：优先使用本地化 description，然后是默认 description
    const description =
      localized?.description ||
      server.description ||
      ''

    return {
      id: server.id,
      providerId: this.id(),
      name: server.id,
      displayName,
      description,
      author: server.publisher,
      downloads: server.view_count,
      likes: 0,
      tags: server.tags,
      category: server.categories?.[0],
      isHosted: false,
      metadata: {
        logoUrl: server.logo_url,
        categories: server.categories,
      },
    }
  }

  private mapToMarketServiceDetail(server: ModelScopeMcpServerDetail, locale: string = 'zh-CN'): MarketServiceDetail {
    const baseService = this.mapToMarketService(server, locale)

    // 获取本地化的 readme
    const getLocalizedReadme = () => {
      // 判断当前语言
      const isZh = locale.startsWith('zh')

      // 优先使用当前语言的 readme
      if (isZh && server.locales?.zh?.readme) {
        return server.locales.zh.readme
      }
      if (!isZh && server.locales?.en?.readme) {
        return server.locales.en.readme
      }

      // 如果当前语言不存在，尝试使用另一个语言
      if (server.locales?.zh?.readme) {
        return server.locales.zh.readme
      }
      if (server.locales?.en?.readme) {
        return server.locales.en.readme
      }

      // 最后使用默认的 readme
      return server.readme
    }

    const envSchema: EnvVarSchema[] | undefined = Array.isArray(server.env_schema)
      ? server.env_schema.map((env) => ({
          name: env.name,
          label: env.description || env.name,
          type: env.type,
          required: env.required,
          default: env.default,
          description: env.description,
        }))
      : undefined

    const transportTypes: TransportType[] = server.supported_transports || ['sse', 'streamable_http']

    const tools = server.tools?.map((tool) => ({
      name: tool.name,
      description: tool.description,
      inputSchema: tool.input_schema,
    }))

    const serverConfig = server.server_config

    return {
      ...baseService,
      readme: getLocalizedReadme(),
      envSchema,
      transportTypes,
      tools,
      serverConfig,
    }
  }
}

export const modelScopeProvider = new ModelScopeProvider()
