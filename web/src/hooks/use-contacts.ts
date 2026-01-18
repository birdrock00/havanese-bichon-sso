import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { get_contacts } from '@/api/search/api';

export const useContacts = (searchTerm: string = "") => {
    const { data: allContacts = [], isLoading, isError } = useQuery({
        queryKey: ['contacts', 'all'],
        queryFn: get_contacts,
        staleTime: 1000 * 60 * 10,
        gcTime: 1000 * 60 * 30,
    });

    const filtered = useMemo(() => {
        if (!searchTerm) return allContacts;
        const lower = searchTerm.toLowerCase();
        return allContacts.filter(email =>
            email.toLowerCase().includes(lower)
        );
    }, [allContacts, searchTerm]);

    return {
        contacts: filtered,
        isLoading,
        isError
    };
};