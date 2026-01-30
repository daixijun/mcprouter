import type {
  MarketProvider,
  ServiceSearchParams,
  ServiceSearchResult,
  MarketServiceDetail,
} from '../types/mcp-market'

export type { MarketProvider }

export interface IMarketProvider extends MarketProvider {}

export abstract class BaseMarketProvider implements MarketProvider {
  abstract id(): string
  abstract name(): string
  abstract iconUrl(): string | undefined
  abstract description(): string | undefined
  abstract searchServices(params: ServiceSearchParams): Promise<ServiceSearchResult>
  abstract getServiceDetail(serviceId: string): Promise<MarketServiceDetail>
}
