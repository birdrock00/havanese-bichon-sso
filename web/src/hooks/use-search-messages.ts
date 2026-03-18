//
// Copyright (c) 2025-2026 rustmailer.com (https://rustmailer.com)
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
import { getRouteApi } from '@tanstack/react-router';
import React from 'react';

const routeApi = getRouteApi('/_authenticated/search/')

export function useSearchMessages() {
    // const queryClient = useQueryClient();
    const search = routeApi.useSearch()
    const navigate = routeApi.useNavigate()

    const page = search.page;
    const pageSize = search.pageSize;
    const sortBy = search.sortBy;
    const sortOrder = search.sortOrder;

    const filter = React.useMemo(() => {
        if (!search.q) return {};
        try {
            return JSON.parse(search.q);
        } catch (e) {
            console.error("URL 'q' parameter parse error:", e);
            return {};
        }
    }, [search.q]);



    const updateParams = React.useCallback((newParams: Partial<typeof search>) => {
        navigate({
            search: (prev) => ({
                ...prev,
                ...newParams,
            }),
            replace: false,
        });
    }, [navigate]);


    const setFilter = React.useCallback((val: any | ((prev: any) => any)) => {
        navigate({
            search: (prev) => {
                let currentFilter = {};
                try {
                    currentFilter = prev.q ? JSON.parse(prev.q) : {};
                } catch (e) {
                    currentFilter = {};
                }
                const nextFilter = typeof val === 'function' ? val(currentFilter) : val;
                return {
                    ...prev,
                    page: 1,
                    q: Object.keys(nextFilter).length > 0 ? JSON.stringify(nextFilter) : undefined
                };
            }
        });
    }, [navigate]);


    const setPage = (p: number) => updateParams({ page: p });

    const setSearchPageSize = (size: number) => {
        localStorage.setItem('bichon_search_page_size', size.toString());
        updateParams({ pageSize: size, page: 1 });
    };

    const setSortBy = (val: "DATE" | "SIZE") => updateParams({ sortBy: val });
    const setSortOrder = (val: "desc" | "asc") => updateParams({ sortOrder: val });

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
        } else {
            setFilter({});
        }
    };

    const reset = () => {
        setFilter({});
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
        setSearchPageSize,
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