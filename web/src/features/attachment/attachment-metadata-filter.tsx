import * as React from "react"
import { ChevronDown } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useSearchContext } from "./context"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { MetadataSelectorField } from "./attachment-metadata-selector"
import { useAttachmentMetadata } from "@/hooks/use-attachment-metadata"

interface MetaFilterProps {
    type: 'extension' | 'category' | 'content_type'
    icon: React.ReactNode
}

export function MetadataFilter({ type, icon }: MetaFilterProps) {
    const { t } = useTranslation()
    const { filter, setFilter } = useSearchContext()
    const [open, setOpen] = React.useState(false)
    const { data: meta, isLoading } = useAttachmentMetadata(open)

    const filterKey = `attachment_${type}` as const
    const currentValue = filter[filterKey] as string

    const optionsMap = {
        extension: meta?.extensions || [],
        category: meta?.categories || [],
        content_type: meta?.content_types || []
    }

    const handleSelect = (value: string | undefined) => {
        setFilter(prev => {
            const next = { ...prev }
            delete next.attachment_extension
            delete next.attachment_category
            delete next.attachment_content_type
            if (value) {
                next[filterKey] = value
            }

            return next
        })
        setOpen(false)
    }

    const handleReset = () => {
        setFilter(prev => {
            const next = { ...prev }
            delete next[filterKey]
            return next
        })
        setOpen(false)
    }

    return (
        <Popover open={open} onOpenChange={setOpen}>
            <PopoverTrigger asChild>
                <Button
                    variant="outline"
                    size="sm"
                    className={cn(
                        "h-6 rounded-none border-l-0 px-3 gap-1.5 transition-colors",
                        currentValue && "bg-primary/10 text-primary hover:bg-primary/20 border-primary/50"
                    )}
                >
                    {icon}
                    <span className="max-w-[80px] truncate">
                        {currentValue || t(`search_more.${type}`)}
                    </span>
                    <ChevronDown className="h-3 w-3 opacity-50 shrink-0" />
                </Button>
            </PopoverTrigger>
            <PopoverContent align="start" className="w-64 p-2 shadow-xl">
                <MetadataSelectorField
                    label={t(`search_more.${type}`)}
                    value={currentValue || ''}
                    options={optionsMap[type]}
                    isLoading={isLoading}
                    onSelect={handleSelect}
                    onReset={handleReset}
                />
            </PopoverContent>
        </Popover>
    )
}