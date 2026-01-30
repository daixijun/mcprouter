export interface MarketProvider {
  id(): string
  name(): string
  iconUrl(): string | undefined
  description(): string | undefined
  searchServices(params: ServiceSearchParams, locale?: string): Promise<ServiceSearchResult>
  getServiceDetail(serviceId: string, locale?: string): Promise<MarketServiceDetail>
}

export interface MarketService {
  id: string
  providerId: string
  name: string
  displayName: string
  description?: string
  author?: string
  downloads?: number
  likes?: number
  tags?: string[]
  category?: string
  isHosted?: boolean
  createdAt?: string
  updatedAt?: string
  metadata?: {
    logoUrl?: string
    categories?: string[]
    [key: string]: any
  }
}

export type TransportType = 'sse' | 'streamable_http' | 'stdio'

export interface EnvVarSchema {
  name: string
  label: string
  type: 'string' | 'number' | 'boolean' | 'secret'
  required: boolean
  default?: string
  description?: string
}

export interface ToolSchema {
  name: string
  description?: string
  inputSchema?: any
}

export interface MarketServiceDetail extends MarketService {
  readme?: string
  envSchema?: EnvVarSchema[]
  transportTypes: TransportType[]
  tools?: ToolSchema[]
  serverConfig?: any
}

export interface ServiceSearchParams {
  query?: string
  category?: string
  tags?: string[]
  sortBy?: 'default' | 'downloads' | 'likes' | 'recent'
  page: number
  pageSize: number
}

export interface ServiceSearchResult {
  services: MarketService[]
  totalCount: number
  page: number
  pageSize: number
  hasMore: boolean
}

export interface ModelScopeMcpServer {
  id: string
  name: string
  chinese_name: string
  description: string
  locales: {
    zh?: { name: string; description: string; readme?: string }
    en?: { name: string; description: string; readme?: string }
  }
  logo_url: string
  publisher: string
  categories: string[]
  tags: string[]
  view_count: number
}

export interface ModelScopeMcpServerDetail extends ModelScopeMcpServer {
  readme?: string
  env_schema?: Array<{
    name: string
    description?: string
    type: 'string' | 'number' | 'boolean' | 'secret'
    required: boolean
    default?: string
  }>
  supported_transports?: ('sse' | 'streamable_http' | 'stdio')[]
  tools?: Array<{
    name: string
    description?: string
    input_schema?: any
  }>
  server_config?: any
}
