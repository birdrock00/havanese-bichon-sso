import axiosInstance from '@/api/axiosInstance'

export interface BrandingInfo {
  company_name: string | null
  tagline: string | null
  logo_url: string | null
}

export interface BrandingUpdate {
  company_name?: string | null
  tagline?: string | null
  logo_mime?: string | null
  logo_base64?: string | null
}

const injectedBase = (window as any).__BICHON_BASE__
const base_url =
  injectedBase === '/' || !injectedBase ? '' : injectedBase

/**
 * Resolves an API path (e.g. the branding logo URL) to a full URL that works
 * both behind a base path and in dev (where the backend runs separately).
 */
export function resolveApiUrl(path: string): string {
  const devBase =
    process.env.NODE_ENV === 'production' ? '' : 'http://localhost:15630'
  const base = (base_url || devBase).replace(/\/+$/, '')
  return `${base}${path}`
}

export async function get_branding(): Promise<BrandingInfo> {
  const { data } = await axiosInstance.get<BrandingInfo>('api/v1/branding')
  return data
}

export async function update_branding(
  payload: BrandingUpdate,
): Promise<BrandingInfo> {
  const { data } = await axiosInstance.put<BrandingInfo>(
    'api/v1/branding',
    payload,
  )
  return data
}

export async function delete_branding_logo(): Promise<BrandingInfo> {
  const { data } = await axiosInstance.delete<BrandingInfo>(
    'api/v1/branding/logo',
  )
  return data
}
