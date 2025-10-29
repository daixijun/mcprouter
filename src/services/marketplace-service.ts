import { invoke } from '@tauri-apps/api/core'
import type {
  MarketplaceService,
  MarketplaceServiceResult,
  McpServerConfig,
} from '../types'

export class MarketplaceApi {
  static async listMarketplaceServices(
    query: string,
    page: number = 1,
    pageSize: number = 20,
  ): Promise<MarketplaceServiceResult> {
    return invoke('list_marketplace_services', {
      query,
      page,
      pageSize,
    })
  }

  static async getMcpServerDetails(
    serviceId: string,
  ): Promise<MarketplaceService> {
    return invoke('get_mcp_server_details', { service_id: serviceId })
  }

  static async installMarketplaceService(
    serviceId: string,
    env_vars?: [string, string][],
  ): Promise<McpServerConfig> {
    return invoke('install_marketplace_service', {
      service_id: serviceId,
      env_vars,
    })
  }
}
