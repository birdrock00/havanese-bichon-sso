import * as React from "react"
import { Check, X } from "lucide-react"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from "@/components/ui/command"
import { cn } from "@/lib/utils"
import { useTranslation } from "react-i18next"

interface MetadataSelectorFieldProps {
    label: string
    value?: string
    options: string[]
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
            opt.toLowerCase().includes(searchTerm.toLowerCase())
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
                        {isLoading && <div className="p-4 text-[10px] text-center opacity-50">{t('common.loading')}</div>}
                        <CommandEmpty className="text-[10px] p-2 text-center">{t('common.noData')}</CommandEmpty>
                        <CommandGroup>
                            {filteredOptions.map((opt) => (
                                <CommandItem
                                    key={opt}
                                    onSelect={() => {
                                        value === opt ? onReset() : onSelect(opt);
                                    }}
                                    className="flex items-center justify-between py-2 px-3 cursor-pointer text-xs"
                                >
                                    <span className="truncate">{opt}</span>
                                    {value === opt && <Check className="h-3 w-3 text-primary shrink-0" />}
                                </CommandItem>
                            ))}
                        </CommandGroup>
                    </CommandList>
                </Command>
            </PopoverContent>
        </Popover>
    )
}