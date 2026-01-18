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


import { get_tags } from '@/api/search/api';
import { useQuery } from '@tanstack/react-query';
import React from 'react';

export interface TagCount {
    tag: string;
    count: number;
}

interface UseAvailableTagsResult {
    tags: string[];
    tagsCount: TagCount[];
    isLoading: boolean;
    isError: boolean;
    error: unknown;
    refetch: () => void;
}


export function useAvailableTags(): UseAvailableTagsResult {
    const {
        data: tagsCount = [],
        isLoading,
        isError,
        error,
        refetch,
    } = useQuery<TagCount[]>({
        queryKey: ['all-tags'],
        queryFn: get_tags,
        staleTime: 60 * 1000,
        retry: false,
        refetchOnWindowFocus: false,
    });

    const tags = React.useMemo(() => {
        return tagsCount.map(f => f.tag).sort();
    }, [tagsCount]);

    return {
        tags,
        tagsCount,
        isLoading,
        isError,
        error,
        refetch,
    };
}