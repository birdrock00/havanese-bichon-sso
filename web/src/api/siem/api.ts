import axiosInstance from '@/api/axiosInstance'

// Enterprise SIEM webhook forwarding config (mirrors the scrubbed
// `SiemConfigView` returned by GET /api/v1/siem — secrets never leave the
// server; the UI only sees whether they are set).

export interface SiemConfigView {
  enabled: boolean
  url: string | null
  auth_token_set: boolean
  hmac_secret_set: boolean
  categories: string[]
  format: string
  mapping_script: string | null
  batch_size: number
  poll_interval_secs: number
  retry_max_attempts: number
  retry_base_delay_secs: number
  retry_max_delay_secs: number
  updated_at: number
}

// Partial update. Secret semantics: omit or send "********" to keep the stored
// secret, "" to clear it, anything else to replace it.
export interface SiemUpdate {
  enabled?: boolean
  url?: string | null
  auth_token?: string
  hmac_secret?: string
  categories?: string[]
  format?: string
  mapping_script?: string | null
  batch_size?: number
  poll_interval_secs?: number
  retry_max_attempts?: number
  retry_base_delay_secs?: number
  retry_max_delay_secs?: number
}

export type SiemFormat = 'generic' | 'splunk-hec' | 'elasticsearch-bulk'

export const SIEM_FORMATS: SiemFormat[] = [
  'generic',
  'splunk-hec',
  'elasticsearch-bulk',
]

export async function get_siem_config(): Promise<SiemConfigView> {
  const { data } = await axiosInstance.get<SiemConfigView>('api/v1/siem')
  return data
}

export async function update_siem_config(
  payload: SiemUpdate,
): Promise<SiemConfigView> {
  const { data } = await axiosInstance.put<SiemConfigView>(
    'api/v1/siem',
    payload,
  )
  return data
}

export interface SiemTestResult {
  ok: boolean
  status?: number
  error?: string
}

export async function test_siem_config(): Promise<SiemTestResult> {
  const { data } = await axiosInstance.post<SiemTestResult>(
    'api/v1/siem/test',
  )
  return data
}

export interface SiemPreview {
  format: string
  content_type: string
  /** The exact body: a JSON object/array for generic/splunk-hec, a raw NDJSON
   *  string for elasticsearch-bulk. */
  body: unknown
  events: number
}

/** Build a preview body without sending anything. Overrides are optional and
 *  fall back to the stored config. */
export async function preview_siem_config(payload: {
  format?: string
  mapping_script?: string | null
}): Promise<SiemPreview> {
  const { data } = await axiosInstance.post<SiemPreview>(
    'api/v1/siem/preview',
    payload,
  )
  return data
}
