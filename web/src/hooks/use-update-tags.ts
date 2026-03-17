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


import { update_tags } from '@/api/search/api';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from './use-toast';

export interface UpdateTagsParams {
    updates: Record<number, string[]>;
    tags: string[];
}

export function useUpdateTags() {
    const queryClient = useQueryClient();

    return useMutation({
        mutationFn: async (params: UpdateTagsParams) => {
            const { updates, tags } = params;
            const payload = {
                updates,
                tags
            };
            return update_tags(payload);
        },

        onSuccess: () => {
            toast({
                title: 'Tags updated',
                description: `Successfully applied to these email(s).`,
            });
            queryClient.invalidateQueries({ queryKey: ['search-messages'], exact: false });
            queryClient.invalidateQueries({ queryKey: ['all-tags'] });
        },
        onError: (error: any) => {
            toast({
                title: 'Failed to update tags',
                description: error?.message || 'Please try again later.',
                variant: 'destructive',
            });
        },
    });
}