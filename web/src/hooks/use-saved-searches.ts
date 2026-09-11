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


import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import {
    create_saved_search,
    delete_saved_search,
    list_saved_searches,
    rename_saved_search,
    SavedSearchKind,
    SavedSearchModel,
} from '@/api/saved-search/api'

export function useSavedSearches(kind: SavedSearchKind) {
    const queryClient = useQueryClient()
    const queryKey = ['saved-searches', kind]

    const { data, isLoading, isError, error } = useQuery<SavedSearchModel[]>({
        queryKey,
        queryFn: () => list_saved_searches(kind),
        staleTime: 30_000,
        retry: false,
    })

    const createMutation = useMutation({
        mutationFn: ({ name, filter }: { name: string; filter: Record<string, any> }) =>
            create_saved_search({ name, kind, filter }),
        onSuccess: () => queryClient.invalidateQueries({ queryKey }),
    })

    const renameMutation = useMutation({
        mutationFn: ({ id, name }: { id: string; name: string }) =>
            rename_saved_search(id, name),
        onSuccess: () => queryClient.invalidateQueries({ queryKey }),
    })

    const deleteMutation = useMutation({
        mutationFn: (id: string) => delete_saved_search(id),
        onSuccess: () => queryClient.invalidateQueries({ queryKey }),
    })

    return {
        searches: data ?? [],
        isLoading,
        isError,
        error,
        createSearch: createMutation,
        renameSearch: renameMutation,
        deleteSearch: deleteMutation,
    }
}