import { useEffect } from 'react'
import { resolveApiUrl } from '@/api/branding/api'
import { useBranding } from '@/hooks/use-branding'

const DEFAULT_TITLE = 'Bichon'

/**
 * Applies the admin-configured branding to browser chrome: tab title and
 * favicon. Rendered once at the root; falls back to the built-in Bichon
 * branding whenever nothing is configured. Colors stay with the per-user
 * Appearance theme — branding is identity only.
 */
export function BrandingApplier() {
  const { branding } = useBranding()

  useEffect(() => {
    if (!branding) return

    const title = branding.company_name?.trim() || DEFAULT_TITLE
    if (document.title !== title) document.title = title

    const link = document.querySelector<HTMLLinkElement>('link[rel="icon"]')
    if (!link) return

    if (branding.logo_url) {
      const href = `${resolveApiUrl(branding.logo_url)}?v=${Date.now()}`
      link.removeAttribute('type')
      link.href = href
    } else {
      link.type = 'image/svg+xml'
      link.href = '/assets/favicon.svg'
    }

  }, [branding])

  return null
}
