import { createFileRoute } from '@tanstack/react-router'
import { z } from 'zod'
import AttachmentSearch from '@/features/attachment'

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

export const Route = createFileRoute('/_authenticated/attachment/')({
  component: AttachmentSearch,
  validateSearch: (search) => {
    const result = searchSchema.parse(search);
    return {
      ...result,
      page: result.page ?? 1,
      pageSize: result.pageSize ?? (Number(localStorage.getItem('bichon_search_attachment_page_size')) || 30),
    }
  }
})
