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
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Separator } from "@/components/ui/separator"
import { ListFilter } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useSearchContext } from "./context"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"
import { Button } from "@/components/ui/button"
import { Badge } from "@/components/ui/badge"
import { Checkbox } from "@/components/ui/checkbox"
import { cn } from "@/lib/utils"
import { useAttachmentMetadata } from "@/hooks/use-attachment-metadata"
import { MetadataSelectorField } from "./attachment-metadata-selector"

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
    const { filter, setFilter } = useSearchContext();
    const [open, setOpen] = React.useState(false);

    const { data: meta, isLoading: metaLoading } = useAttachmentMetadata(open);

    const [localState, setLocalState] = React.useState({
        attachment_name: filter?.attachment_name || '',
        attachment_extension: filter?.attachment_extension || '',
        attachment_category: filter?.attachment_category || '',
        attachment_content_type: filter?.attachment_content_type || '',
        message_id: filter?.message_id || '',
        size_preset: getPresetFromSize(filter?.min_size, filter?.max_size),
        has_attachment: filter?.has_attachment || false
    });

    React.useEffect(() => {
        if (open) {
            setLocalState({
                attachment_name: filter?.attachment_name || '',
                attachment_extension: filter?.attachment_extension || '',
                attachment_category: filter?.attachment_category || '',
                attachment_content_type: filter?.attachment_content_type || '',
                message_id: filter?.message_id || '',
                size_preset: getPresetFromSize(filter?.min_size, filter?.max_size),
                has_attachment: filter?.has_attachment || false
            });
        }
    }, [open, filter]);

    const handleApply = () => {
        setFilter(prev => {
            const next = { ...prev };

            if (localState.attachment_name) next.attachment_name = localState.attachment_name;
            else delete next.attachment_name;

            if (localState.attachment_extension) next.attachment_extension = localState.attachment_extension;
            else delete next.attachment_extension;

            if (localState.attachment_category) next.attachment_category = localState.attachment_category;
            else delete next.attachment_category;

            if (localState.attachment_content_type) next.attachment_content_type = localState.attachment_content_type;
            else delete next.attachment_content_type;

            if (localState.message_id) next.message_id = localState.message_id;
            else delete next.message_id;

            if (localState.has_attachment) next.has_attachment = true;
            else delete next.has_attachment;

            const range = SIZES[localState.size_preset as keyof typeof SIZES] || { min: undefined, max: undefined };
            if (range.min) next.min_size = range.min; else delete next.min_size;
            if (range.max) next.max_size = range.max; else delete next.max_size;

            return next;
        });
        setOpen(false);
    };

    const activeCount = [
        filter?.attachment_name,
        filter?.min_size,
        filter?.max_size,
        filter?.message_id,
        filter?.has_attachment,
        filter?.attachment_extension,
        filter?.attachment_category,
        filter?.attachment_content_type
    ].filter(Boolean).length;

    return (
        <Popover open={open} onOpenChange={setOpen}>
            <PopoverTrigger asChild>
                <Button
                    variant="outline"
                    size="sm"
                    className={cn(
                        "h-8 gap-2 px-3 rounded-none border-l-0",
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
                                    delete next.attachment_name;
                                    delete next.min_size;
                                    delete next.max_size;
                                    delete next.message_id;
                                    delete next.has_attachment;
                                    delete next.attachment_extension;
                                    delete next.attachment_category;
                                    delete next.attachment_content_type;
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
                        id="has_attachment"
                        checked={localState.has_attachment}
                        onCheckedChange={(checked) => {
                            const isChecked = checked as boolean;
                            setLocalState(prev => ({
                                ...prev,
                                has_attachment: isChecked,
                                ...(isChecked ? {} : {
                                    attachment_name: '',
                                    attachment_extension: '',
                                    attachment_category: '',
                                    attachment_content_type: ''
                                })
                            }));
                        }}
                    />
                    <Label
                        htmlFor="has_attachment"
                        className="text-xs font-normal cursor-pointer select-none"
                    >
                        {t('search_more.has_attachment')}
                    </Label>
                </div>

                {localState.has_attachment && (
                    <div className="space-y-3 p-2 bg-muted/30 rounded-lg border border-dashed border-border animate-in fade-in slide-in-from-top-1">
                        <MetadataSelectorField
                            label={t('search_more.extension')}
                            value={localState.attachment_extension}
                            options={meta?.extensions || []}
                            isLoading={metaLoading}
                            onSelect={(v) => setLocalState(p => ({ ...p, attachment_extension: v, attachment_category: '', attachment_content_type: '' }))}
                            onReset={() => setLocalState(p => ({ ...p, attachment_extension: '' }))}
                        />

                        <MetadataSelectorField
                            label={t('search_more.category')}
                            value={localState.attachment_category}
                            options={meta?.categories || []}
                            isLoading={metaLoading}
                            onSelect={(v) => setLocalState(p => ({ ...p, attachment_category: v, attachment_extension: '', attachment_content_type: '' }))}
                            onReset={() => setLocalState(p => ({ ...p, attachment_category: '' }))}
                        />

                        <MetadataSelectorField
                            label={t('search_more.content_types')}
                            value={localState.attachment_content_type}
                            options={meta?.content_types || []}
                            isLoading={metaLoading}
                            onSelect={(v) => setLocalState(p => ({ ...p, attachment_content_type: v, attachment_extension: '', attachment_category: '' }))}
                            onReset={() => setLocalState(p => ({ ...p, attachment_content_type: '' }))}
                        />

                        <div className="space-y-1 px-1">
                            <Label className="text-xs text-muted-foreground">{t('search_more.attachment_name_label')}</Label>
                            <Input
                                className="h-8 text-xs"
                                value={localState.attachment_name}
                                onChange={(e) => setLocalState(prev => ({ ...prev, attachment_name: e.target.value }))}
                                placeholder={t('search_more.attachment_name_placeholder')}
                            />
                        </div>
                    </div>
                )}

                <div className="space-y-2">
                    <Label className="text-xs text-muted-foreground">{t('search_more.message_size_label')}</Label>
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

                <div className="space-y-2">
                    <Label className="text-xs text-muted-foreground">{t('search_more.message_id_label')}</Label>
                    <Input
                        className="h-8 text-xs"
                        value={localState.message_id}
                        onChange={(e) => setLocalState(prev => ({ ...prev, message_id: e.target.value }))}
                    />
                    <p className="text-[10px] text-muted-foreground opacity-70 leading-tight">
                        {t('search_more.message_id_description')}
                    </p>
                </div>

                <Button size="sm" className="w-full h-8 text-xs mt-2" onClick={handleApply}>
                    {t('search_more.apply')}
                </Button>
            </PopoverContent>
        </Popover>
    );
}