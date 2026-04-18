import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { get_attachment_senders } from '@/api/attachment/api';

export const userAttachmentSenders = (searchTerm: string = "") => {
    const { data: senders = [], isLoading, isError } = useQuery({
        queryKey: ['attachment-senders', 'all'],
        queryFn: get_attachment_senders,
        staleTime: 1000 * 60 * 10,
        gcTime: 1000 * 60 * 30,
    });

    const filtered = useMemo(() => {
        if (!searchTerm) return senders;
        const lower = searchTerm.toLowerCase();
        return senders.filter(email =>
            email.toLowerCase().includes(lower)
        );
    }, [senders, searchTerm]);

    return {
        senders: filtered,
        isLoading,
        isError
    };
};