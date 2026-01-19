import { X } from "lucide-react"
import { Button } from "@/components/ui/button"
import { useSearchContext } from "./context"
import { cn } from "@/lib/utils"
import { useTranslation } from "react-i18next";

export function FilterResetButton() {
    const { filter, setFilter } = useSearchContext();
    const { t } = useTranslation()
    const { q, ...restFilters } = filter;
    
    const activeFiltersCount = Object.keys(restFilters).filter(key => {
        const value = restFilters[key];
        if (Array.isArray(value)) return value.length > 0;
        return value !== undefined && value !== null && value !== '';
    }).length;

    if (activeFiltersCount === 0) return null;

    return (
        <Button
            variant="ghost"
            size="sm"
            onClick={() => setFilter(q ? { q } : {})}
            className={cn(
                "h-8 px-2 text-xs gap-1.5 font-normal",
                "text-muted-foreground hover:text-destructive hover:bg-destructive/10 transition-colors"
            )}
            title={t('search_reset.tooltip')}
        >
            <span>{t('search_reset.label')}</span>
            <div className="flex items-center justify-center w-4 h-4 rounded-full bg-muted-foreground/20 text-[10px]">
                {activeFiltersCount}
            </div>
            <X className="h-3 w-3" />
        </Button>
    );
}