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
import { type ThemeSwatch } from '@/lib/theme'
import { cn } from '@/lib/utils'

/** `sm`: fixed-size tile for the header popover; `lg`: fills a settings card. */
type PreviewSize = 'sm' | 'lg'

/** A miniature window drawn in a theme's colors. */
export function ThemePreview({
  swatch,
  size,
}: {
  swatch: ThemeSwatch
  size: PreviewSize
}) {
  const { background, color: primary, muted } = swatch

  if (size === 'sm') {
    return (
      <div
        className='flex h-10 w-20 shrink-0 flex-col gap-1.5 rounded-md border p-1.5'
        style={{ backgroundColor: background, borderColor: muted }}
      >
        <div className='flex gap-1'>
          <span
            className='h-1 w-1 rounded-full'
            style={{ backgroundColor: muted }}
          />
          <span
            className='h-1 w-1 rounded-full'
            style={{ backgroundColor: muted }}
          />
          <span
            className='h-1 w-1 rounded-full'
            style={{ backgroundColor: primary }}
          />
        </div>
        <div
          className='h-1.5 w-12 rounded-sm'
          style={{ backgroundColor: primary }}
        />
        <div
          className='h-1 w-16 rounded-sm opacity-70'
          style={{ backgroundColor: muted }}
        />
        <div
          className='h-1 w-10 rounded-sm opacity-50'
          style={{ backgroundColor: muted }}
        />
      </div>
    )
  }

  return (
    <div
      className='flex h-12 w-full flex-col gap-1.5 rounded-md border p-2 shadow-sm'
      style={{ backgroundColor: background, borderColor: 'transparent' }}
    >
      <div className='flex gap-1'>
        <span
          className='h-1.5 w-1.5 rounded-full'
          style={{ backgroundColor: muted }}
        />
        <span
          className='h-1.5 w-1.5 rounded-full'
          style={{ backgroundColor: primary }}
        />
      </div>
      <div
        className='h-1.5 w-3/4 rounded-sm'
        style={{ backgroundColor: primary }}
      />
      <div
        className='h-1.5 w-1/2 rounded-sm opacity-50'
        style={{ backgroundColor: muted }}
      />
    </div>
  )
}

/** Light and dark variants of one family, split along the diagonal. */
export function SplitThemePreview({
  light,
  dark,
  size,
}: {
  light: ThemeSwatch
  dark: ThemeSwatch
  size: PreviewSize
}) {
  return (
    <div className={cn('relative', size === 'sm' && 'shrink-0')}>
      <ThemePreview swatch={light} size={size} />
      <div
        className='absolute inset-0'
        style={{ clipPath: 'polygon(100% 0, 100% 100%, 0 100%)' }}
      >
        <ThemePreview swatch={dark} size={size} />
      </div>
    </div>
  )
}
