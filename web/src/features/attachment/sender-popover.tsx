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

import * as React from "react"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { ChevronDown, Mail } from "lucide-react"
import { useTranslation } from 'react-i18next'
import { useSearchContext } from "./context"
import { userAttachmentSenders } from "@/hooks/use-attachment-senders"
import { Group } from "@/api/system/api"
import { MetadataSelectorField } from "./attachment-metadata-selector"

export function SenderFilterPopover() {
    const { t } = useTranslation()
    const { filter, setFilter } = useSearchContext()
    const { senders, isLoading } = userAttachmentSenders("")

    const activeCount = filter.from ? 1 : 0
    const senderOptions: Group[] = React.useMemo(() => {
        return senders.map(email => ({
            key: email,
            count: 0
        }))
    }, [senders])

    const updateFilter = (email: string | undefined) => {
        setFilter(prev => ({
            ...prev,
            from: email
        }))
    }

    return (
        <Popover>
            <PopoverTrigger asChild>
                <Button
                    size="sm"
                    variant="outline"
                    className={cn(
                        'h-6 rounded-none px-3 gap-1.5 transition-colors border-l-0',
                        activeCount > 0 && 'bg-primary/10 text-primary hover:bg-primary/20'
                    )}
                >
                    <Mail className="h-3.5 w-3.5 opacity-60" />
                    <span>
                        {activeCount > 0
                            ? t('attachment.sender_with_count', { count: activeCount })
                            : t('attachment.sender')}
                    </span>
                    <ChevronDown className="h-3.5 w-3.5 opacity-60" />
                </Button>
            </PopoverTrigger>

            <PopoverContent
                align="start"
                className="w-fit min-w-[280px] max-w-[90vw] sm:max-w-[min(90vw,500px)] p-0 flex flex-col divide-y divide-border shadow-xl"
            >
                <div className="flex flex-col bg-muted/20">
                    <MetadataSelectorField
                        label={t('attachment.sender')}
                        value={filter.from}
                        options={senderOptions}
                        isLoading={isLoading}
                        onSelect={(val) => updateFilter(val)}
                        onReset={() => updateFilter(undefined)}
                    />
                </div>
            </PopoverContent>
        </Popover>
    )
}