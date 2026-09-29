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


import { createFileRoute } from '@tanstack/react-router'
import EmailSearch from '@/features/search'
import { z } from 'zod'

const searchSchema = z.object({
  page: z.number().catch(1),
  pageSize: z.number().optional(),
  // Optional: when absent the backend defaults to RELEVANCE when the query
  // carries a text term, else DATE. Don't fall back to DATE here, or an unset
  // URL param would always send an explicit sort_by and defeat the default.
  sortBy: z.enum(['DATE', 'SIZE', 'RELEVANCE']).optional(),
  sortOrder: z.enum(['asc', 'desc']).catch('desc'),
  q: z.string().optional(),
})

export const Route = createFileRoute('/_authenticated/search/')({
  component: EmailSearch,
  validateSearch: (search) => {
    const result = searchSchema.parse(search);
    return {
      ...result,
      page: result.page ?? 1,
      pageSize: result.pageSize ?? (Number(localStorage.getItem('bichon_search_page_size')) || 30),
    }
  }
})
