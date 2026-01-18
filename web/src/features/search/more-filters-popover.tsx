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

    const [localState, setLocalState] = React.useState({
        attachment_name: filter?.attachment_name || '',
        message_id: filter?.message_id || '',
        size_preset: getPresetFromSize(filter?.min_size, filter?.max_size),
        has_attachment: filter?.has_attachment || false
    });

    React.useEffect(() => {
        if (open) {
            setLocalState({
                attachment_name: filter?.attachment_name || '',
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
        filter?.has_attachment
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
                    <span className="text-xs">Advanced</span>
                    {activeCount > 0 && (
                        <Badge className="ml-1 h-4 px-1 text-[10px] bg-primary text-primary-foreground border-none rounded-sm">
                            {activeCount}
                        </Badge>
                    )}
                </Button>
            </PopoverTrigger>

            <PopoverContent align="end" className="w-72 p-4 flex flex-col gap-4">
                <div className="flex items-center justify-between">
                    <h4 className="text-xs font-medium">Advanced Filters</h4>
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
                                    return next;
                                });
                                setOpen(false);
                            }}
                        >
                            Reset
                        </Button>
                    )}
                </div>
                <Separator />
                <div className="flex items-center space-x-2 px-1">
                    <Checkbox
                        id="has_attachment"
                        checked={localState.has_attachment}
                        onCheckedChange={(checked) =>
                            setLocalState(prev => ({ ...prev, has_attachment: checked as boolean }))
                        }
                    />
                    <Label
                        htmlFor="has_attachment"
                        className="text-xs font-normal cursor-pointer select-none"
                    >
                        Has Attachments
                    </Label>
                </div>

                <div className="space-y-2">
                    <Label className="text-xs text-muted-foreground">Attachment Name</Label>
                    <Input
                        className="h-8 text-xs"
                        value={localState.attachment_name}
                        onChange={(e) => setLocalState(prev => ({ ...prev, attachment_name: e.target.value }))}
                        placeholder="e.g. invoice.pdf"
                    />
                </div>

                <div className="space-y-2">
                    <Label className="text-xs text-muted-foreground">Message Size</Label>
                    <Select
                        value={localState.size_preset}
                        onValueChange={(v) => setLocalState(prev => ({ ...prev, size_preset: v }))}
                    >
                        <SelectTrigger className="h-8 text-xs">
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem className="text-xs" value="any">{t('search.any')}</SelectItem>
                            <SelectItem className="text-xs" value="tiny">{t('search.tiny')}</SelectItem>
                            <SelectItem className="text-xs" value="small">{t('search.small')}</SelectItem>
                            <SelectItem className="text-xs" value="medium">{t('search.medium')}</SelectItem>
                            <SelectItem className="text-xs" value="large">{t('search.large')}</SelectItem>
                            <SelectItem className="text-xs" value="huge">{t('search.huge')}</SelectItem>
                        </SelectContent>
                    </Select>
                </div>

                <div className="space-y-2">
                    <Label className="text-xs text-muted-foreground">Original Message ID</Label>
                    <Input
                        className="h-8 text-xs"
                        value={localState.message_id}
                        onChange={(e) => setLocalState(prev => ({ ...prev, message_id: e.target.value }))}
                    />
                    <p className="text-[10px] text-muted-foreground opacity-70 leading-tight">
                        {t('search.originalMessageIdHeader')}
                    </p>
                </div>

                <Button size="sm" className="w-full h-8 text-xs mt-2" onClick={handleApply}>
                    Apply Filters
                </Button>
            </PopoverContent>
        </Popover>
    );
}