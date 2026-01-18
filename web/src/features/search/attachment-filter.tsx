import { Paperclip, Check } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useSearchContext } from './context'
import { Button } from '@/components/ui/button'
import { cn } from '@/lib/utils'

export function AttachmentFilter() {
    const { t } = useTranslation()
    const { filter, setFilter } = useSearchContext()

    const hasAttachment = filter?.has_attachment === true

    const toggleAttachment = () => {
        setFilter((prev) => {
            const next = { ...prev }
            if (next.has_attachment) {
                delete next.has_attachment
            } else {
                next.has_attachment = true
            }
            return next
        })
    }

    return (
        <Button
            size="sm"
            variant="outline"
            onClick={toggleAttachment}
            className={cn(
                "h-8 px-3 gap-2 transition-all rounded-none flex-shrink-0",
                hasAttachment
                    ? "bg-primary/10 border-primary text-primary hover:bg-primary/20 hover:text-primary z-10"
                    : "text-muted-foreground border-r-0"
            )}
        >
            <Paperclip
                className={cn(
                    "h-3.5 w-3.5",
                    hasAttachment ? "opacity-100" : "opacity-60"
                )}
            />

            <span className="text-xs font-medium">
                {t('mail.attachments')}
            </span>

            {hasAttachment && (
                <Check className="h-3 w-3 ml-0.5 stroke-[3px] animate-in zoom-in duration-200" />
            )}
        </Button>
    )
}