import { get_attachment_meta } from '@/api/mailbox/envelope/api';
import { useQuery } from '@tanstack/react-query';

export const ATTACHMENT_METADATA_KEY = ['attachment_metadata'] as const;

export const useAttachmentMetadata = (enabled = true) => {
    return useQuery({
        queryKey: ATTACHMENT_METADATA_KEY,
        queryFn: get_attachment_meta,
        staleTime: 1000 * 60 * 10,
        gcTime: 1000 * 60 * 30,
        enabled: enabled
    });
};