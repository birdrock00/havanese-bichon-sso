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
import { Check, X } from "lucide-react"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from "@/components/ui/command"
import { cn } from "@/lib/utils"
import { useTranslation } from "react-i18next"
import { Group } from "@/api/system/api"

interface MetadataSelectorFieldProps {
    label: string
    value?: string
    options: Group[]
    isLoading: boolean
    onSelect: (val: string | undefined) => void
    onReset: () => void
}

export function MetadataSelectorField({
    label,
    value,
    options,
    isLoading,
    onSelect,
    onReset
}: MetadataSelectorFieldProps) {
    const { t } = useTranslation()
    const [searchTerm, setSearchTerm] = React.useState("")

    const filteredOptions = React.useMemo(() => {
        return options.filter(opt =>
            opt.key.toLowerCase().includes(searchTerm.toLowerCase())
        )
    }, [options, searchTerm])

    return (
        <Popover>
            <PopoverTrigger asChild>
                <button
                    className={cn(
                        "group flex items-center justify-between w-full px-4 py-2 hover:bg-accent/50 transition-all text-left relative border rounded-md",
                        "min-h-[48px]",
                        value && "bg-accent/30 border-primary/50"
                    )}
                >
                    <div className="flex flex-col items-start pr-6 overflow-hidden">
                        <span className="text-[10px] font-bold uppercase opacity-50 tracking-tight leading-none">
                            {label}
                        </span>
                        <span className={cn(
                            "mt-1 truncate w-full text-xs",
                            value ? "font-semibold text-primary" : "text-muted-foreground/70"
                        )}>
                            {value || t('search_more.any')}
                        </span>
                    </div>

                    <div className="flex items-center gap-1.5">
                        {value && (
                            <div
                                onClick={(e) => { e.stopPropagation(); onReset(); }}
                                className="p-1 rounded-full hover:bg-destructive/10 text-muted-foreground hover:text-destructive transition-colors"
                            >
                                <X className="h-3 w-3" />
                            </div>
                        )}
                    </div>
                    {value && <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-primary" />}
                </button>
            </PopoverTrigger>

            <PopoverContent align="start" className="p-0 w-64 shadow-xl">
                <Command shouldFilter={false}>
                    <CommandInput
                        placeholder={t('search_more.search_placeholder', { field: label })}
                        value={searchTerm}
                        onValueChange={setSearchTerm}
                        className="h-8"
                    />
                    <CommandList className="max-h-[240px]">
                        {isLoading && (
                            <div className="p-4 text-[10px] text-center opacity-50">
                                {t('common.loading')}
                            </div>
                        )}
                        <CommandEmpty className="text-[10px] p-2 text-center">
                            {t('common.noData')}
                        </CommandEmpty>

                        <CommandGroup>
                            {filteredOptions.map((opt) => (
                                <CommandItem
                                    key={opt.key}
                                    onSelect={() => {
                                        value === opt.key ? onReset() : onSelect(opt.key)
                                    }}
                                    className="flex items-center justify-between py-2 px-3 cursor-pointer text-xs"
                                >
                                    <span className="truncate">{opt.key}</span>

                                    <div className="flex items-center gap-2">
                                        {/* count */}
                                        <span className="text-[10px] text-muted-foreground">
                                            {opt.count}
                                        </span>

                                        {value === opt.key && (
                                            <Check className="h-3 w-3 text-primary shrink-0" />
                                        )}
                                    </div>
                                </CommandItem>
                            ))}
                        </CommandGroup>
                    </CommandList>
                </Command>
            </PopoverContent>
        </Popover>
    )
}