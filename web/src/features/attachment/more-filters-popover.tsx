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
import { Label } from "@/components/ui/label"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Separator } from "@/components/ui/separator"
import { Info, ListFilter } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useAttachmentContext } from "./context"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Checkbox } from "@/components/ui/checkbox"
import { cn } from "@/lib/utils"
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip"

const SIZES = {
    tiny: { min: undefined, max: 15 * 1024 },
    small: { min: undefined, max: 2 * 1024 * 1024 },
    medium: { min: 2 * 1024 * 1024, max: 10 * 1024 * 1024 },
    large: { min: 10 * 1024 * 1024, max: 20 * 1024 * 1024 },
    huge: { min: 20 * 1024 * 1024, max: undefined },
};

const getPresetFromSize = (min?: number, max?: number) => {
    if (min === SIZES.huge.min) return 'huge';
    if (min === SIZES.large.min && max === SIZES.large.max) return 'large';
    if (min === SIZES.medium.min && max === SIZES.medium.max) return 'medium';
    if (!min && max === SIZES.small.max) return 'small';
    if (!min && max === SIZES.tiny.max) return 'tiny';
    return 'any';
};

export function MoreFiltersPopover() {
    const { t } = useTranslation();
    const { filter, setFilter } = useAttachmentContext();
    const [open, setOpen] = React.useState(false);

    const [localState, setLocalState] = React.useState({
        size_preset: getPresetFromSize(filter?.min_size, filter?.max_size),
        is_message: filter?.is_message || false
    });

    React.useEffect(() => {
        if (open) {
            setLocalState({
                size_preset: getPresetFromSize(filter?.min_size, filter?.max_size),
                is_message: filter?.is_message || false
            });
        }
    }, [open, filter]);

    const handleApply = () => {
        setFilter(prev => {
            const next = { ...prev };

            if (localState.is_message) next.is_message = true;
            else delete next.is_message;

            const range = SIZES[localState.size_preset as keyof typeof SIZES] || { min: undefined, max: undefined };
            if (range.min) next.min_size = range.min; else delete next.min_size;
            if (range.max) next.max_size = range.max; else delete next.max_size;

            return next;
        });
        setOpen(false);
    };

    const activeCount = [
        filter?.min_size,
        filter?.max_size,
        filter?.is_message,
    ].filter(Boolean).length;

    return (
        <Popover open={open} onOpenChange={setOpen}>
            <PopoverTrigger asChild>
                <Button
                    variant="outline"
                    size="sm"
                    className={cn(
                        "h-6 gap-2 px-3 rounded-none border-l-0",
                        activeCount > 0 && "bg-primary/10 border-primary text-primary"
                    )}
                >
                    <ListFilter className="h-3.5 w-3.5" />
                    <span className="text-xs">{t('search_more.trigger_label')}</span>
                    {activeCount > 0 && (
                        <Badge className="ml-1 h-4 px-1 text-[10px] bg-primary text-primary-foreground border-none rounded-sm">
                            {activeCount}
                        </Badge>
                    )}
                </Button>
            </PopoverTrigger>

            <PopoverContent align="end" className="w-72 p-4 flex flex-col gap-4">
                <div className="flex items-center justify-between">
                    <h4 className="text-xs font-medium">{t('search_more.title')}</h4>
                    {activeCount > 0 && (
                        <Button
                            variant="ghost"
                            className="h-auto p-0 text-[10px] text-muted-foreground hover:text-destructive"
                            onClick={() => {
                                setFilter(prev => {
                                    const next = { ...prev };
                                    delete next.min_size;
                                    delete next.max_size;
                                    delete next.is_message;
                                    return next;
                                });
                                setOpen(false);
                            }}
                        >
                            {t('search_more.reset')}
                        </Button>
                    )}
                </div>
                <Separator />
                <div className="flex items-center space-x-2 px-1">
                    <Checkbox
                        id="is_message"
                        checked={localState.is_message}
                        onCheckedChange={(checked) => {
                            const isChecked = checked as boolean;
                            setLocalState(prev => ({
                                ...prev,
                                is_message: isChecked
                            }));
                        }}
                    />
                    <Label
                        htmlFor="is_message"
                        className="text-xs font-normal cursor-pointer select-none"
                    >
                        {t('search_more.is_message')}
                    </Label>
                    <TooltipProvider>
                        <Tooltip>
                            <TooltipTrigger asChild>
                                <Info className="w-3 h-3 ml-1.5 text-muted-foreground cursor-help" />
                            </TooltipTrigger>
                            <TooltipContent>
                                <p className="max-w-xs">{t('search_more.is_message_desc')}</p>
                            </TooltipContent>
                        </Tooltip>
                    </TooltipProvider>
                </div>
                <div className="space-y-2">
                    <Label className="text-xs text-muted-foreground">{t('attachment.size')}</Label>
                    <Select
                        value={localState.size_preset}
                        onValueChange={(v) => setLocalState(prev => ({ ...prev, size_preset: v }))}
                    >
                        <SelectTrigger className="h-8 text-xs">
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            {Object.keys(SIZES).concat('any').map((key) => (
                                <SelectItem key={key} className="text-xs" value={key}>
                                    {t(`search_more.size_presets.${key}`)}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                </div>

                <Button size="sm" className="w-full h-8 text-xs mt-2" onClick={handleApply}>
                    {t('search_more.apply')}
                </Button>
            </PopoverContent>
        </Popover>
    );
}