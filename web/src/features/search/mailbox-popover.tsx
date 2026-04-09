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


import * as React from 'react';
import {
    ChevronDown, Folders, X, TreeDeciduous, FolderIcon,
    MoreVertical, Trash2, Search,
    Check
} from 'lucide-react';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useTranslation } from 'react-i18next';
import { animated, useSpring } from '@react-spring/web';
import { styled } from '@mui/material/styles';
import Collapse from '@mui/material/Collapse';
import { TransitionProps } from '@mui/material/transitions';

import {
    TreeItemCheckbox,
    TreeItemContent,
    TreeItemDragAndDropOverlay,
    TreeItemIcon,
    TreeItemIconContainer,
    TreeItemLabel,
    TreeItemProvider,
    TreeItemRoot,
    useTreeItemModel,
} from '@mui/x-tree-view';
import { RichTreeView } from '@mui/x-tree-view/RichTreeView';
import { useTreeItem, UseTreeItemParameters } from '@mui/x-tree-view/useTreeItem';

import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Input } from '@/components/ui/input';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { cn } from '@/lib/utils';

import { list_mailboxes } from '@/api/mailbox/api';
import useMinimalAccountList from '@/hooks/use-minimal-account-list';
import { useSearchContext } from './context';
import { buildTree, ExtendedTreeItemProps } from '@/lib/build-tree';

const CustomCollapse = styled(Collapse)({ padding: 0 });
const AnimatedCollapse = animated(CustomCollapse);

function TransitionComponent(props: TransitionProps) {
    const style = useSpring({
        to: {
            opacity: props.in ? 1 : 0,
            transform: `translate3d(0,${props.in ? 0 : 20}px,0)`,
        },
    });
    return <AnimatedCollapse style={style} {...props} />;
}

interface CustomTreeItemProps
    extends Omit<UseTreeItemParameters, 'rootRef'>,
    Omit<React.HTMLAttributes<HTMLLIElement>, 'onFocus'> { }

interface CustomLabelProps {
    exists?: number;
    attributes?: { attr: string; extension: string | null }[],
    children: React.ReactNode;
    id: string;
    icon?: React.ElementType;
    expandable?: boolean;
    onDelete: (id: string) => void;
}

function CustomLabel({
    expandable,
    exists,
    attributes,
    children,
    id,
    onDelete,
    ...other
}: CustomLabelProps) {
    const { t } = useTranslation()
    return (
        <TreeItemLabel
            {...other}
            sx={{
                display: 'flex',
                alignItems: 'center',
            }}
        >
            <FolderIcon className="mr-2 h-3.5 w-3.5" />
            <span className="font-medium text-xs text-inherit">
                {children}
            </span>
            <div className="ml-auto flex items-center">
                <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                        <Button
                            variant="ghost"
                            size="icon"
                            className="h-6 w-6 p-0 hover:bg-muted rounded-md"
                            onMouseDown={(e) => e.stopPropagation()}
                            onClick={(e) => {
                                e.stopPropagation();
                                e.preventDefault();
                            }}
                        >
                            <MoreVertical className="h-4 w-4" />
                        </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end" className="w-24">
                        <DropdownMenuItem
                            className="text-destructive focus:text-destructive flex items-center px-2 py-1 text-[11px] cursor-pointer"
                            onClick={(e) => {
                                e.stopPropagation();
                            }}
                            onSelect={(e) => {
                                e.preventDefault();
                                onDelete(id);
                            }}
                        >
                            <Trash2 className="mr-1 h-3 w-3" />
                            <span>{t('common.delete')}</span>
                        </DropdownMenuItem>
                    </DropdownMenuContent>
                </DropdownMenu>
            </div>
        </TreeItemLabel>
    );
}

export function MailboxPopover() {
    const { t } = useTranslation();
    const { filter, setFilter, setOpen, setDeleteMailboxId, setSelectedAccountId } = useSearchContext();
    const { minimalList = [] } = useMinimalAccountList();

    const [localOpen, setLocalOpen] = React.useState(false);
    const [search, setSearch] = React.useState('');

    const accountIds: number[] = filter.account_ids ?? [];
    const selectedMailboxIds: number[] = filter.mailbox_ids ?? [];

    const [localSelectedIds, setLocalSelectedIds] = React.useState<number[]>([]);
    const [activeAccountId, setActiveAccountId] = React.useState<number | undefined>(undefined);

    const queryClient = useQueryClient();

    React.useEffect(() => {
        if (localOpen) {
            const globalMailboxIds = filter.mailbox_ids ?? [];
            setLocalSelectedIds(globalMailboxIds);

            const currentAccountIds = filter.account_ids ?? [];
            if (currentAccountIds.length > 0) {
                if (!activeAccountId || !currentAccountIds.includes(activeAccountId)) {
                    setActiveAccountId(currentAccountIds[0]);
                }
            } else {
                setActiveAccountId(undefined);
            }
        }
    }, [localOpen, activeAccountId, filter.account_ids, filter.mailbox_ids]);

    const { data: activeMailboxes = [], isLoading: activeIsLoading } = useQuery({
        queryKey: ['search-mailboxes', activeAccountId],
        queryFn: () => list_mailboxes(activeAccountId!, false),
        enabled: !!activeAccountId,      
    });

    const treeData = React.useMemo(() => {
        const filtered = search.trim()
            ? activeMailboxes.filter(m => m.name.toLowerCase().includes(search.toLowerCase()))
            : activeMailboxes;
        return buildTree(filtered);
    }, [activeMailboxes, search]);

    const disabled = accountIds.length === 0;

    const handleApply = () => {
        setFilter(prev => ({
            ...prev,
            mailbox_ids: localSelectedIds.length > 0 ? localSelectedIds : undefined
        }));
        setLocalOpen(false);
    };

    const handleDeleteClick = (id: string) => {
        setDeleteMailboxId(id);
        setSelectedAccountId(activeAccountId);
        setOpen('delete-mailbox');
    };


    const CustomTreeItem = React.forwardRef(function CustomTreeItem(
        props: CustomTreeItemProps,
        ref: React.Ref<HTMLLIElement>,
    ) {
        const { id, itemId, label, disabled, children, ...other } = props;
        const {
            getContextProviderProps,
            getRootProps,
            getContentProps,
            getLabelProps,
            getIconContainerProps,
            getCheckboxProps,
            getGroupTransitionProps,
            getDragAndDropOverlayProps,
            status,
        } = useTreeItem({ id, itemId, children, label, disabled, rootRef: ref });

        const item = useTreeItemModel<ExtendedTreeItemProps>(itemId)!;


        return (
            <TreeItemProvider {...getContextProviderProps()}>
                <TreeItemRoot {...getRootProps(other)} className="group">
                    <TreeItemContent {...getContentProps()} sx={{ paddingY: '2px' }}>
                        <TreeItemIconContainer {...getIconContainerProps()}>
                            <TreeItemIcon status={status} />
                        </TreeItemIconContainer>
                        <TreeItemCheckbox {...getCheckboxProps()} sx={{
                            color: 'hsl(var(--muted-foreground) / 0.4)',
                            '&.Mui-checked': {
                                color: 'hsl(var(--primary))',
                            },
                            '& .MuiSvgIcon-root': {
                                fontSize: '1.3rem'
                            }
                        }} />
                        <CustomLabel
                            {...getLabelProps({
                                exists: item.exists,
                                id: item.id,
                                onDelete: handleDeleteClick,
                                attributes: item.attributes,
                                expandable: status.expandable && status.expanded,
                            })}
                        />

                        <TreeItemDragAndDropOverlay {...getDragAndDropOverlayProps()} />
                    </TreeItemContent>
                    {children && <TransitionComponent {...getGroupTransitionProps()} />}
                </TreeItemRoot>
            </TreeItemProvider>
        );
    });

    return (
        <Popover open={localOpen} onOpenChange={setLocalOpen} >
            <PopoverTrigger asChild>
                <Button
                    size="sm"
                    variant="outline"
                    disabled={disabled}
                    className={cn(
                        'h-8 rounded-none px-3 gap-1.5 transition-colors',
                        selectedMailboxIds.length > 0 && 'bg-primary/10 text-primary border-primary/20'
                    )}
                >
                    <Folders className="h-4 w-4" />
                    <span className="max-w-[100px] truncate">{t('search_mailbox.label')}</span>
                    {selectedMailboxIds.length > 0 && (
                        <span className="flex h-4 w-4 items-center justify-center rounded-full bg-primary text-[10px] text-primary-foreground">
                            {selectedMailboxIds.length}
                        </span>
                    )}
                    <ChevronDown className="h-3 w-3 opacity-50" />
                </Button>
            </PopoverTrigger>

            <PopoverContent
                align="start"
                className="w-[740px] max-w-[95vw] p-0 flex flex-col h-[480px] shadow-xl border-muted"
            >
                <div className="flex items-center gap-2 p-2 border-b bg-muted/10">
                    <div className="relative flex-1">
                        <Search className="absolute left-2.5 top-2.5 h-3.5 w-3.5 text-muted-foreground" />
                        <Input
                            value={search}
                            onChange={e => setSearch(e.target.value)}
                            placeholder={t('search_mailbox.search_placeholder')}
                            className="h-9 pl-8 text-xs bg-background"
                        />
                    </div>
                    {localSelectedIds.length > 0 && (
                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => setLocalSelectedIds([])}
                            className="h-9 text-xs text-destructive hover:bg-destructive/10"
                        >
                            <X className="mr-1.5 h-3 w-3" />
                            {t('common.clear')}
                        </Button>
                    )}
                </div>

                <div className="flex flex-1 min-h-0">
                    <div className="w-64 border-r bg-muted/20 flex flex-col">
                        <ScrollArea className="flex-1">
                            <div className="p-2 space-y-1">
                                {accountIds.map(id => {
                                    const acc = minimalList.find(a => a.id === id);
                                    const isActive = activeAccountId === id;
                                    const cachedData = queryClient.getQueryData<any[]>(['search-mailboxes', id]);
                                    const count = cachedData?.filter(m => localSelectedIds.includes(m.id)).length ?? 0;

                                    return (
                                        <button
                                            key={id}
                                            onClick={() => setActiveAccountId(id)}
                                            className={cn(
                                                "w-full flex items-center justify-between px-3 py-2 text-left rounded-md transition-all",
                                                isActive
                                                    ? "bg-background shadow-sm text-primary ring-1 ring-black/5"
                                                    : "text-muted-foreground hover:bg-muted/50 hover:text-foreground"
                                            )}
                                        >
                                            <span className="text-xs truncate font-medium">
                                                {acc?.email}
                                            </span>
                                            {count > 0 && (
                                                <span className="text-[10px] font-bold bg-primary/10 px-1.5 py-0.5 rounded-full">
                                                    {count}
                                                </span>
                                            )}
                                        </button>
                                    );
                                })}
                            </div>
                        </ScrollArea>
                    </div>
                    <div className="flex-1 flex flex-col bg-background">
                        <ScrollArea className="flex-1">
                            <div className="p-3">
                                {activeIsLoading ? (
                                    <div className="p-4 space-y-4">
                                        {[1, 2, 3, 4, 5].map(i => (
                                            <div key={i} className="h-3 bg-muted animate-pulse rounded w-full" />
                                        ))}
                                    </div>
                                ) : activeAccountId ? (
                                    <RichTreeView
                                        multiSelect
                                        items={treeData}
                                        checkboxSelection
                                        expansionTrigger="iconContainer"
                                        selectedItems={localSelectedIds.map(String)}
                                        onSelectedItemsChange={(_, itemIds) => {
                                            setLocalSelectedIds(itemIds.map(id => parseInt(id)).filter(id => !isNaN(id)));
                                        }}
                                        slots={{ item: CustomTreeItem }}
                                        sx={{ width: '100%' }}
                                    />
                                ) : (
                                    <div className="flex flex-col items-center justify-center h-64 text-muted-foreground opacity-40">
                                        <TreeDeciduous className="h-12 w-12 mb-2 stroke-[1px]" />
                                        <p className="text-xs">{t('search_mailbox.select_account_tip')}</p>
                                    </div>
                                )}
                            </div>
                        </ScrollArea>

                        <div className="p-3 border-t bg-muted/10 flex items-center justify-between">
                            <div className="text-[10px] text-muted-foreground font-medium">
                                {t('search_mailbox.selected_total')}: <span className="text-foreground">{localSelectedIds.length}</span>
                            </div>
                            <div className="flex gap-2">
                                <Button variant="ghost" size="sm" onClick={() => setLocalOpen(false)} className="h-8 px-3 text-xs">
                                    {t('common.cancel')}
                                </Button>
                                <Button size="sm" onClick={handleApply} className="h-8 px-4 text-xs gap-1.5 shadow-sm">
                                    <Check className="h-3.5 w-3.5" />
                                    {t('common.apply')}
                                </Button>
                            </div>
                        </div>
                    </div>
                </div>
            </PopoverContent>
        </Popover >
    );
}