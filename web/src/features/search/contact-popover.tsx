import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { useSearchContext } from "./context"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { Check, ChevronDown, Mail, X } from "lucide-react"
import React from "react"
import { useContacts } from "@/hooks/use-contacts"
import {
    Command,
    CommandEmpty,
    CommandGroup,
    CommandInput,
    CommandItem,
    CommandList,
} from "@/components/ui/command"

export function MailFilterPopover() {
    const { filter, setFilter } = useSearchContext()
    const fields = ['from', 'to', 'cc', 'bcc'] as const

    const activeCount = fields.filter(k => !!filter[k]).length

    const updateFilter = (field: string, email: string | undefined) => {
        setFilter(prev => ({
            ...prev,
            [field]: email
        }))
    }

    const resetAll = () => {
        setFilter(prev => {
            const next = { ...prev }
            fields.forEach(k => delete next[k])
            return next
        })
    }

    return (
        <Popover>
            <PopoverTrigger asChild>
                <Button
                    size="sm"
                    variant="outline"
                    className={cn(
                        'h-8 rounded-none px-3 gap-1.5 transition-colors',
                        activeCount > 0 && 'bg-primary/10 text-primary hover:bg-primary/20'
                    )}
                >
                    <Mail className="h-3.5 w-3.5 opacity-60" />
                    <span>{activeCount > 0 ? `Participants (${activeCount})` : 'Participants'}</span>
                    <ChevronDown className="h-3.5 w-3.5 opacity-60" />
                </Button>
            </PopoverTrigger>

            <PopoverContent
                align="start"
                className="w-fit min-w-[280px] max-w-[90vw] sm:max-w-[min(90vw,500px)] p-0 flex flex-col divide-y divide-border shadow-xl"
            >
                <div className="flex flex-col bg-muted/20 divide-y divide-border">
                    {fields.map((field) => (
                        <ContactSelectorField
                            key={field}
                            label={field}
                            value={filter[field] as string | undefined}
                            onSelect={(email) => updateFilter(field, email)}
                            onReset={() => updateFilter(field, undefined)}
                        />
                    ))}
                </div>

                {activeCount > 0 && (
                    <div className="p-2 flex justify-end bg-background">
                        <Button
                            variant="ghost"
                            size="sm"
                            className="h-7 px-3 text-xs font-medium text-muted-foreground hover:text-destructive transition-colors"
                            onClick={resetAll}
                        >
                            Reset All Participants
                        </Button>
                    </div>
                )}
            </PopoverContent>
        </Popover>
    )
}

function ContactSelectorField({
    label,
    value,
    onSelect,
    onReset
}: {
    label: string
    value?: string
    onSelect: (email: string | undefined) => void
    onReset: () => void
}) {
    const [searchTerm, setSearchTerm] = React.useState("")
    const { contacts, isLoading } = useContacts(searchTerm)

    const handleToggle = (email: string) => {
        if (value === email) {
            onReset()
        } else {
            onSelect(email)
        }
    }

    return (
        <Popover>
            <PopoverTrigger asChild>
                <button
                    className={cn(
                        "group flex items-center justify-between w-full px-4 py-3 hover:bg-background transition-all text-left relative",
                        "min-h-[52px]",
                        value && "bg-background/60 hover:bg-background/80"
                    )}
                >
                    <div className="flex flex-col items-start pr-6">
                        <span className="text-[10px] font-bold uppercase opacity-50 tracking-tight leading-none">
                            {label}
                        </span>
                        <span
                            className={cn(
                                "mt-0.5 truncate max-w-[320px]",
                                value
                                    ? "text-xs font-semibold text-primary"
                                    : "text-xs text-muted-foreground/90"
                            )}
                        >
                            {value || 'Any'}
                        </span>
                    </div>

                    <div className="flex items-center gap-1.5 opacity-0 group-hover:opacity-100 transition-opacity">
                        {value && (
                            <button
                                type="button"
                                onClick={(e) => {
                                    e.stopPropagation()
                                    onReset()
                                }}
                                className="p-1 rounded-full hover:bg-destructive/10 text-muted-foreground hover:text-destructive"
                            >
                                <X className="h-3.5 w-3.5" />
                            </button>
                        )}
                    </div>

                    {value && (
                        <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-primary" />
                    )}
                </button>
            </PopoverTrigger>

            <PopoverContent
                side="right"
                align="start"
                className="p-0 w-auto min-w-[300px] max-w-[420px] shadow-2xl border-border/50"
            >
                <Command shouldFilter={false}>
                    <CommandInput
                        placeholder={`Search ${label}...`}
                        className="h-9"
                        value={searchTerm}
                        onValueChange={setSearchTerm}
                    />
                    <CommandList className="max-h-[360px]">
                        {isLoading && (
                            <div className="p-4 text-xs text-center opacity-50">Loading...</div>
                        )}
                        <CommandEmpty>No contact found.</CommandEmpty>
                        <CommandGroup>
                            {contacts.slice(0, 100).map((email) => (
                                <CommandItem
                                    key={email}
                                    onSelect={() => handleToggle(email)}
                                    className="flex items-center justify-between py-2.5 px-3 cursor-pointer whitespace-nowrap gap-4 text-xs"
                                >
                                    <div className="flex flex-col min-w-0">
                                        <span className="font-medium">
                                            {email.split('@')[0]}
                                        </span>
                                        <span className="text-[10px] text-muted-foreground truncate max-w-[360px]">
                                            {email}
                                        </span>
                                    </div>
                                    {value === email && (
                                        <Check className="h-4 w-4 text-primary shrink-0" />
                                    )}
                                </CommandItem>
                            ))}
                            {contacts.length > 100 && (
                                <div className="px-3 py-2 text-[10px] text-center text-muted-foreground border-t border-border/50">
                                    Showing top 100 results • {contacts.length} total
                                </div>
                            )}
                        </CommandGroup>
                    </CommandList>
                </Command>
            </PopoverContent>
        </Popover>
    )
}