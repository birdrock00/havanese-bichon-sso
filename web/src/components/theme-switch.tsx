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

import { useEffect } from 'react'
import { Moon, Sun, Check } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { Theme, useTheme } from '@/context/theme-context'
import { Button } from '@/components/ui/button'
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover'

import { cn } from '@/lib/utils'
import {
  THEME_FAMILIES,
  type ThemeMode,
  themeFamily,
  themeMode,
  themeVariant,
} from '@/lib/theme'
import { SplitThemePreview, ThemePreview } from '@/components/theme-preview'
import { Label } from '@/components/ui/label'
import { Switch } from '@/components/ui/switch'

function ThemeItem({
  labelKey,
  preview,
  active,
  onSelect,
}: {
  labelKey: string
  preview: React.ReactNode
  active: boolean
  onSelect: () => void
}) {
  const { t } = useTranslation()

  return (
    <button
      onClick={onSelect}
      className={cn(
        'flex w-full items-center gap-2 rounded-lg px-2 py-1.5 transition-colors',
        active ? 'bg-accent' : 'hover:bg-accent/50'
      )}
    >
      {preview}

      <span className='flex-1 text-left text-xs font-medium'>
        {t(labelKey)}
      </span>

      <Check
        className={cn(
          'size-4 shrink-0 transition-opacity',
          active ? 'opacity-100' : 'opacity-0'
        )}
      />
    </button>
  )
}

function ThemeColumn({
  icon,
  label,
  mode,
  isActive,
  onSelect,
}: {
  icon: React.ReactNode
  label: string
  mode: ThemeMode
  isActive: (v: Theme) => boolean
  onSelect: (v: Theme) => void
}) {
  return (
    <div className='space-y-1'>
      <div className='mb-2 flex items-center gap-1.5 px-1 text-[11px] font-medium uppercase tracking-wider text-muted-foreground/60'>
        {icon}
        {label}
      </div>

      {THEME_FAMILIES.map((family) => {
        const variant = family[mode]
        return (
          <ThemeItem
            key={variant.value}
            labelKey={family.labelKey}
            preview={<ThemePreview swatch={variant} size='sm' />}
            active={isActive(variant.value)}
            onSelect={() => onSelect(variant.value)}
          />
        )
      })}
    </div>
  )
}

export function ThemeSwitch() {
  const { setTheme, followSystem, setFollowSystem, resolvedTheme } = useTheme()
  const { t } = useTranslation()

  const isDark = themeMode(resolvedTheme) === 'dark'

  // Mark the applied variant (equals `theme` unless following the system).
  const isActive = (v: Theme) => v === resolvedTheme

  useEffect(() => {
    document
      .querySelector("meta[name='theme-color']")
      ?.setAttribute('content', themeVariant(resolvedTheme).metaColor)
  }, [resolvedTheme])

  return (
    <Popover>
      <PopoverTrigger asChild>
        <Button variant='ghost' size='icon' className='rounded-full'>
          {isDark ? (
            <Moon className='size-5' />
          ) : (
            <Sun className='size-5' />
          )}
        </Button>
      </PopoverTrigger>

      <PopoverContent align='end' className='w-[420px] p-3'>
        <p className='mb-3 px-1 text-[11px] font-medium uppercase tracking-wider text-muted-foreground/60'>
          {t('theme.appearance')}
        </p>

        <div className='mb-3 flex items-center justify-between rounded-lg px-1'>
          <Label htmlFor='theme-follow-system' className='text-xs font-medium'>
            {t('theme.use_system')}
          </Label>
          <Switch
            id='theme-follow-system'
            checked={followSystem}
            onCheckedChange={setFollowSystem}
          />
        </div>

        {followSystem ? (
          // One item per family; picking it keeps the variant currently shown.
          <div className='grid grid-cols-2 gap-x-3 gap-y-1'>
            {THEME_FAMILIES.map((family) => (
              <ThemeItem
                key={family.id}
                labelKey={family.labelKey}
                preview={<SplitThemePreview light={family.light} dark={family.dark} size='sm' />}
                active={family.id === themeFamily(resolvedTheme)}
                onSelect={() => setTheme(family[themeMode(resolvedTheme)].value)}
              />
            ))}
          </div>
        ) : (
          <div className='grid grid-cols-2 gap-3'>
            <ThemeColumn
              icon={<Sun className='size-3.5' />}
              label={t('theme.light')}
              mode='light'
              isActive={isActive}
              onSelect={setTheme}
            />

            <div className='border-l border-border/40 pl-3'>
              <ThemeColumn
                icon={<Moon className='size-3.5' />}
                label={t('theme.dark')}
                mode='dark'
                isActive={isActive}
                onSelect={setTheme}
              />
            </div>
          </div>
        )}
      </PopoverContent>
    </Popover>
  )
}
