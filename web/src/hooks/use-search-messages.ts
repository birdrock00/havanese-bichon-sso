//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
//
// This file is part of the Bichon Email Archiving Project
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.


import { EmailEnvelope, PaginatedResponse } from '@/api';
import { search_messages } from '@/api/search/api';
import { useQuery } from '@tanstack/react-query';
import { useState } from 'react';



export function useSearchMessages() {
    // const queryClient = useQueryClient();
    const [filter, setFilter] = useState<Record<string, any>>({});
    const [page, setPage] = useState(1);
    const [pageSize, setPageSize] = useState(30);
    const [sortBy, setSortBy] = useState<"DATE" | "SIZE">("DATE");
    const [sortOrder, setSortOrder] = useState<"desc" | "asc">("desc");

    const onSubmit = (cleaned: Record<string, any>) => {
        if ('has_attachment' in cleaned && cleaned.has_attachment === false) {
            delete cleaned.has_attachment;
        }
        if (Object.keys(cleaned).length > 0) {
            const payload = {
                ...cleaned,
                ...(cleaned.since && { since: cleaned.since.getTime() }),
                ...(cleaned.before && { before: cleaned.before.getTime() }),
            };
            setFilter(payload);
            setPage(1);
        } else {
            setFilter({});
        }
    };

    const reset = () => {
        setFilter({});
        setPage(1);
    }

    const {
        data,
        isLoading,
        isError,
        error,
        isFetching,
    } = useQuery<PaginatedResponse<EmailEnvelope>>({
        queryKey: ['search-messages', filter, page, pageSize, sortBy, sortOrder],
        queryFn: () =>
            search_messages({
                filter: filter,
                page,
                page_size: pageSize,
                sort_by: sortBy,
                desc: sortOrder === "desc"
            }),
        staleTime: 1000,
        retry: false,
    });

    return {
        emails: data?.items ?? [],
        total: data?.total_items ?? 0,
        totalPages: data?.total_pages ?? 1,
        pageSize: data?.page_size ?? pageSize,
        setPageSize,
        sortBy,
        setSortBy,
        sortOrder,
        setSortOrder,
        isLoading,
        isError,
        error: error as Error | null,
        isFetching,
        page,
        setPage,
        onSubmit,
        reset,
        filter,
        setFilter
    };
}