import { createFileRoute } from '@tanstack/react-router'
import { z } from 'zod'
import AttachmentSearch from '@/features/attachment'

const searchSchema = z.object({
  page: z.number().catch(1),
  pageSize: z.number().optional(),
  sortBy: z.enum(['DATE', 'SIZE']).catch('DATE'),
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
