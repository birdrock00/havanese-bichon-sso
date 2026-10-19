import axiosInstance from '@/api/axiosInstance'

// SMTP journaling: the archive inlet. Journaling delivers a copy of every
// message the tenant's mail system handles to one address, before delivery.
// The source network is the credential (journal deliveries cannot
// authenticate), so the server refuses to enable the feature without at least
// one allowed CIDR.

export interface JournalConfigView {
  enabled: boolean
  target_address: string
  allowed_cidrs: string[]
}

export async function getJournalConfig(): Promise<JournalConfigView> {
  const { data } = await axiosInstance.get<JournalConfigView>(
    'api/v1/journal/config'
  )
  return data
}

export async function updateJournalConfig(
  payload: JournalConfigView
): Promise<JournalConfigView> {
  const { data } = await axiosInstance.post<JournalConfigView>(
    'api/v1/journal/config',
    payload
  )
  return data
}
