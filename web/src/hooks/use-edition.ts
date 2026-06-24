import axiosInstance from '@/api/axiosInstance'
import { useQuery } from '@tanstack/react-query'

export interface EditionInfo {
  features: string[]
  edition: 'community' | 'pro' | 'enterprise'
  version: string
}

async function fetchEdition(): Promise<EditionInfo> {
  const { data } = await axiosInstance.get<EditionInfo>('api/v1/features')
  return data
}

export function useEdition() {
  const { data } = useQuery({
    queryKey: ['edition'],
    queryFn: fetchEdition,
    staleTime: Infinity,
    retry: 1,
  })

  return {
    isPro: data?.edition === 'pro' || data?.edition === 'enterprise',
    edition: data?.edition ?? 'community',
    features: data?.features ?? [],
  } as const
}
