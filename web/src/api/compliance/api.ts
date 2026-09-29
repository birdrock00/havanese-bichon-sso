import axiosInstance from '@/api/axiosInstance'

// Pro/Enterprise compliance configuration (audit retention, integrity
// schedule/retention, TSP anchoring interval). Values are stored as
// WebUI-configured overrides on top of the env/CLI defaults; `null` means no
// override is stored (the env/CLI default is in effect).

export interface ComplianceConfigView {
  audit_retention_days: number | null
  integrity_schedule_hours: number | null
  integrity_retention_days: number | null
  tsp_interval_hours: number | null
}

// Partial update. `undefined` leaves a field unchanged, `''` clears it
// (returning to the env/CLI default), a number string sets it.
export interface ComplianceConfigUpdate {
  audit_retention_days?: string
  integrity_schedule_hours?: string
  integrity_retention_days?: string
  tsp_interval_hours?: string
}

export async function getComplianceConfig(): Promise<ComplianceConfigView> {
  const { data } = await axiosInstance.get<ComplianceConfigView>(
    'api/v1/compliance/config'
  )
  return data
}

export async function updateComplianceConfig(
  payload: ComplianceConfigUpdate
): Promise<ComplianceConfigView> {
  const { data } = await axiosInstance.post<ComplianceConfigView>(
    'api/v1/compliance/config',
    payload
  )
  return data
}
