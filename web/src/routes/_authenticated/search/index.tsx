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
import Search from '@/features/search'
import { z } from 'zod'

const searchSchema = z.object({
  page: z.number().catch(1),
  pageSize: z.number().optional(),
  sortBy: z.enum(['DATE', 'SIZE']).catch('DATE'),
  sortOrder: z.enum(['asc', 'desc']).catch('desc'),
  q: z.string().optional(),
})

export const Route = createFileRoute('/_authenticated/search/')({
  component: Search,
  validateSearch: (search) => {
    const result = searchSchema.parse(search);
    return {
      ...result,
      page: result.page ?? 1,
      pageSize: result.pageSize ?? (Number(localStorage.getItem('bichon_search_page_size')) || 30),
    }
  }
})
