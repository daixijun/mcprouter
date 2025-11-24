import { invoke } from '@tauri-apps/api/core'
import type { DashboardStats } from '../types'

export class DashboardService {
  static async getDashboardStats(
    forceRefresh?: boolean,
  ): Promise<DashboardStats> {
    return invoke('get_dashboard_stats', {
      force_refresh: forceRefresh,
    })
  }

}
