import React, { useState, useEffect, useRef } from "react"
import { Input } from "@/components/ui/input"
import { Button } from "@/components/ui/button"
import { Search, X, Clock, Trash2 } from "lucide-react"
import { cn } from "@/lib/utils"
import { useSearchContext } from "./context"

const STORAGE_KEY = "mail_search_history"
const MAX_HISTORY = 20

export function TextSearchInput() {
    const { filter, setFilter } = useSearchContext()
    const [value, setValue] = useState(filter.text || "")
    const [history, setHistory] = useState<string[]>([])
    const [showHistory, setShowHistory] = useState(false)
    const inputRef = useRef<HTMLInputElement>(null)
    const containerRef = useRef<HTMLDivElement>(null)

    useEffect(() => {
        try {
            const saved = localStorage.getItem(STORAGE_KEY)
            if (saved) {
                setHistory(JSON.parse(saved))
            }
        } catch (err) {
            console.warn("Failed to load search history", err)
        }
    }, [])

    useEffect(() => {
        setValue(filter.text || "")
    }, [filter.text])

    const saveToHistory = (term: string) => {
        if (!term.trim()) return

        setHistory(prev => {
            const trimmed = term.trim()
            const withoutCurrent = prev.filter(item => item !== trimmed)
            const newHistory = [trimmed, ...withoutCurrent].slice(0, MAX_HISTORY)

            try {
                localStorage.setItem(STORAGE_KEY, JSON.stringify(newHistory))
            } catch (err) {
                console.warn("Failed to save search history", err)
            }

            return newHistory
        })
    }

    const handleSearch = () => {
        const trimmed = value.trim()
        setFilter(prev => ({
            ...prev,
            text: trimmed || undefined
        }))
        if (trimmed) {
            saveToHistory(trimmed)
        }
        setShowHistory(false)
        inputRef.current?.blur()
    }

    const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (e.key === "Enter") {
            e.preventDefault()
            handleSearch()
        }
    }

    const handleClear = () => {
        setValue("")
        setFilter(prev => {
            const next = { ...prev }
            delete next.text
            return next
        })
        setShowHistory(false)
    }

    const handleSelectHistory = (term: string) => {
        setValue(term)
        setShowHistory(false)
    }

    const handleClearHistory = () => {
        setHistory([])
        try {
            localStorage.removeItem(STORAGE_KEY)
        } catch (err) {
            console.warn("Failed to clear search history", err)
        }
        setShowHistory(false)
    }

    useEffect(() => {
        const handleClickOutside = (e: MouseEvent) => {
            if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
                setShowHistory(false)
            }
        }
        document.addEventListener("mousedown", handleClickOutside)
        return () => document.removeEventListener("mousedown", handleClickOutside)
    }, [])

    const isActive = !!filter.text?.trim()

    return (
        <div ref={containerRef} className="relative w-full max-w-[550px] min-w-[280px]">
            <div className="relative flex items-center gap-1.5">
                <div className="relative flex-1">
                    <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                    <Input
                        ref={inputRef}
                        value={value}
                        onChange={(e) => setValue(e.target.value)}
                        onFocus={() => setShowHistory(true)}
                        onKeyDown={handleKeyDown}
                        placeholder='Search messages... (use "double quotes" for exact phrases)'
                        className={cn(
                            "h-9 pl-9 pr-9 text-sm",
                            isActive && "border-primary/50 focus-visible:ring-primary/30"
                        )}
                    />
                    {value && (
                        <Button
                            variant="ghost"
                            size="icon"
                            className="absolute right-1 top-1/2 h-7 w-7 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                            onClick={handleClear}
                        >
                            <X className="h-4 w-4" />
                        </Button>
                    )}
                </div>

                <Button
                    size="sm"
                    className="h-9 px-5"
                    onClick={handleSearch}
                    disabled={!value.trim()}
                >
                    Search
                </Button>
            </div>

            {showHistory && (
                <div className="absolute top-full left-0 w-full mt-1 bg-popover border rounded-md shadow-md z-50 max-h-[280px] overflow-auto">
                    <div className="py-1.5 px-3 text-xs text-muted-foreground font-medium border-b flex items-center justify-between">
                        <div className="flex items-center gap-1.5">
                            <Clock className="h-3 w-3" />
                            Recent searches
                        </div>
                        {history.length > 0 && (
                            <button
                                onClick={handleClearHistory}
                                className="text-xs text-destructive hover:text-destructive/80 flex items-center gap-1 hover:underline"
                            >
                                <Trash2 className="h-3 w-3" />
                                Clear all
                            </button>
                        )}
                    </div>

                    {history.length > 0 ? (
                        history.map((term, idx) => (
                            <button
                                key={idx}
                                className="w-full text-left px-3 py-2 text-xs hover:bg-accent transition-colors flex items-center gap-2"
                                onClick={() => handleSelectHistory(term)}
                            >
                                <Search className="h-3.5 w-3.5 text-muted-foreground" />
                                {term}
                            </button>
                        ))
                    ) : (
                        <div className="px-3 py-4 text-xs text-center text-muted-foreground">
                            No recent searches
                        </div>
                    )}
                </div>
            )}
        </div>
    )
}