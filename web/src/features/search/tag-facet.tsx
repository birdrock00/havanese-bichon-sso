//
// Copyright (c) 2025 rustmailer.com (https://rustmailer.com)
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


import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from '@/components/ui/collapsible';
import { ChevronDown, ChevronUp, Tag } from 'lucide-react';
import React from 'react';
import { useAvailableTags } from '@/hooks/use-available-tags';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Label } from '@/components/ui/label';
import { useTranslation } from 'react-i18next';

interface EnvelopeTagsProps {
  selectedTags: string[];
  onTagToggle: (tag: string) => void;
}

export function EnvelopeTags({ selectedTags, onTagToggle }: EnvelopeTagsProps) {
  const { t } = useTranslation()
  const [open, setOpen] = React.useState(true);

  const {
    tagsCount: tagsCount = [],
    isLoading: tagsIsLoading,
  } = useAvailableTags();

  const sortedTags = React.useMemo(() => {
    return [...tagsCount].sort((a, b) => b.count - a.count);
  }, [tagsCount]);

  if (tagsIsLoading) {
    return (
      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <div className="h-4 w-32 bg-muted animate-pulse rounded" />
        </div>
        <div className="space-y-1.5">
          {[...Array(6)].map((_, i) => (
            <div key={i} className="flex items-center gap-3 px-2 py-1.5">
              <div className="h-4 w-4 bg-muted animate-pulse rounded" />
              <div className="h-4 flex-1 bg-muted animate-pulse rounded" />
              <div className="h-5 w-10 bg-muted animate-pulse rounded" />
            </div>
          ))}
        </div>
      </div>
    );
  }

  return (
    <Collapsible open={open} onOpenChange={setOpen} className="space-y-2">
      <CollapsibleTrigger className="flex w-full items-center justify-between text-sm font-medium hover:text-primary transition-colors">
        <div className="flex items-center gap-2">
          <Tag className="w-4 h-4" />
          {t('mail.tags')}
          {selectedTags.length > 0 && (
            <Badge variant="secondary" className="ml-1.5 h-5 px-1.5 text-xs">
              {selectedTags.length}
            </Badge>
          )}
        </div>
        {open ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
      </CollapsibleTrigger>

      <CollapsibleContent className="space-y-0">
        {sortedTags.length === 0 ? (
          <p className="py-2 pl-2 text-sm text-muted-foreground">{t('mail.noTagsYet')}</p>
        ) : (
          <ScrollArea className="h-[45rem] w-full pr-4 -mr-4">
            {sortedTags.map(({ tag: facet, count }) => {
              const checked = selectedTags.includes(facet);
              const id = `tag-${facet}`;

              return (
                <div
                  key={facet}
                  className="flex items-center gap-3 px-2 py-0.5 hover:bg-accent/80 rounded-md transition-colors cursor-pointer group"
                  onClick={() => onTagToggle(facet)}
                >
                  <Checkbox
                    id={id}
                    checked={checked}
                    onCheckedChange={() => onTagToggle(facet)}
                    onClick={(e) => e.stopPropagation()}
                    className="h-4 w-4"
                  />
                  <Label
                    htmlFor={id}
                    className="flex-1 max-w-[140px] lg:max-w-[120px] cursor-pointer truncate text-sm font-medium"
                    title={facet}
                  >
                    {facet}
                  </Label>
                  <div className="shrink-0 ml-2">
                    <Badge
                      variant="secondary"
                      className="h-5 px-1.5 text-xs font-medium min-w-[1.75rem] text-center"
                    >
                      {count}
                    </Badge>
                  </div>
                </div>
              );
            })}
          </ScrollArea>
        )}
      </CollapsibleContent>
    </Collapsible>
  );
}