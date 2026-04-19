import { get_envelope } from '@/api/mailbox/envelope/api';
import { useQuery } from '@tanstack/react-query';

export const useEnvelope = (
    accountId: number | undefined,
    envelopeId: string | undefined
) => {
    return useQuery({
        queryKey: ['envelope', accountId, envelopeId],
        queryFn: () => get_envelope(accountId!, envelopeId!),
        enabled: !!accountId && !!envelopeId,
        staleTime: 10000,
        retry: false
    });
};