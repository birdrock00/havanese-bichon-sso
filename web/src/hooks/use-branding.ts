import { useQuery } from '@tanstack/react-query'
import { get_branding } from '@/api/branding/api'

export function useBranding() {
  const { data } = useQuery({
    queryKey: ['branding'],
    queryFn: get_branding,
    staleTime: Infinity,
    retry: 1,
  })

  return {
    branding: data ?? null,
    companyName: data?.company_name ?? null,
    tagline: data?.tagline ?? null,
    logoUrl: data?.logo_url ?? null,
  } as const
}
