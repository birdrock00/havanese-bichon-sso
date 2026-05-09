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

const themeColors: Record<Theme, string> = {
  'light': '#ffffff',
  'dark': '#020817',
  'rose-light': '#fb7185',
  'rose-dark': '#881337',
  'orange-light': '#fb923c',
  'orange-dark': '#9a3412',
  'green-light': '#4ade80',
  'green-dark': '#166534',
  'yellow-light': '#facc15',
  'yellow-dark': '#a16207',
  'blue-light': '#60a5fa',
  'blue-dark': '#1e40af',
}

const LIGHT_THEMES = [
  {
    value: 'light',
    labelKey: 'theme.default',
    background: '#ffffff',
    color: '#0f172a',
    muted: '#cbd5e1',
  },
  {
    value: 'rose-light',
    labelKey: 'theme.rose',
    background: '#fff1f2',
    color: '#e11d48',
    muted: '#fda4af',
  },
  {
    value: 'orange-light',
    labelKey: 'theme.orange',
    background: '#fff7ed',
    color: '#ea580c',
    muted: '#fdba74',
  },
  {
    value: 'green-light',
    labelKey: 'theme.green',
    background: '#f0fdf4',
    color: '#16a34a',
    muted: '#86efac',
  },
  {
    value: 'yellow-light',
    labelKey: 'theme.yellow',
    background: '#fefce8',
    color: '#ca8a04',
    muted: '#fde047',
  },
  {
    value: 'blue-light',
    labelKey: 'theme.blue',
    background: '#eff6ff',
    color: '#2563eb',
    muted: '#bfdbfe',
  },
] as const

const DARK_THEMES = [
  {
    value: 'dark',
    labelKey: 'theme.default',
    background: '#020817',
    color: '#94a3b8',
    muted: '#334155',
  },
  {
    value: 'rose-dark',
    labelKey: 'theme.rose',
    background: '#1f1115',
    color: '#e11d48',
    muted: '#881337',
  },
  {
    value: 'orange-dark',
    labelKey: 'theme.orange',
    background: '#1c120d',
    color: '#f97316',
    muted: '#9a3412',
  },
  {
    value: 'green-dark',
    labelKey: 'theme.green',
    background: '#0d1b12',
    color: '#22c55e',
    muted: '#166534',
  },
  {
    value: 'yellow-dark',
    labelKey: 'theme.yellow',
    background: '#1c1608',
    color: '#facc15',
    muted: '#a16207',
  },
  {
    value: 'blue-dark',
    labelKey: 'theme.blue',
    background: '#172554',
    color: '#60a5fa',
    muted: '#1e3a8a',
  }
] as const

type ThemeOption =
  | (typeof LIGHT_THEMES)[number]
  | (typeof DARK_THEMES)[number]

function ThemePreview({
  background,
  primary,
  muted,
}: {
  background: string
  primary: string
  muted: string
}) {
  return (
    <div
      className='flex h-10 w-20 shrink-0 flex-col gap-1.5 rounded-md border p-1.5'
      style={{
        backgroundColor: background,
        borderColor: muted,
      }}
    >
      <div className='flex gap-1'>
        <span className='h-1 w-1 rounded-full' style={{ backgroundColor: muted }} />
        <span className='h-1 w-1 rounded-full' style={{ backgroundColor: muted }} />
        <span className='h-1 w-1 rounded-full' style={{ backgroundColor: primary }} />
      </div>
      <div className='h-1.5 w-12 rounded-sm' style={{ backgroundColor: primary }} />
      <div className='h-1 w-16 rounded-sm opacity-70' style={{ backgroundColor: muted }} />
      <div className='h-1 w-10 rounded-sm opacity-50' style={{ backgroundColor: muted }} />
    </div>
  )
}

function ThemeItem({
  item,
  active,
  onSelect,
}: {
  item: ThemeOption
  active: boolean
  onSelect: (v: Theme) => void
}) {
  const { t } = useTranslation()

  return (
    <button
      onClick={() => onSelect(item.value as Theme)}
      className={cn(
        'flex w-full items-center gap-2 rounded-lg px-2 py-1.5 transition-colors',
        active ? 'bg-accent' : 'hover:bg-accent/50'
      )}
    >
      <ThemePreview
        background={item.background}
        primary={item.color}
        muted={item.muted}
      />

      <span className='flex-1 text-left text-xs font-medium'>
        {t(item.labelKey)}
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
  items,
  currentTheme,
  onSelect,
}: {
  icon: React.ReactNode
  label: string
  items: readonly ThemeOption[]
  currentTheme: Theme
  onSelect: (v: Theme) => void
}) {
  return (
    <div className='space-y-1'>
      <div className='mb-2 flex items-center gap-1.5 px-1 text-[11px] font-medium uppercase tracking-wider text-muted-foreground/60'>
        {icon}
        {label}
      </div>

      {items.map((item) => (
        <ThemeItem
          key={item.value}
          item={item}
          active={currentTheme === item.value}
          onSelect={onSelect}
        />
      ))}
    </div>
  )
}

export function ThemeSwitch() {
  const { theme, setTheme } = useTheme()
  const { t } = useTranslation()

  const isDark = theme.includes('dark')

  useEffect(() => {
    document
      .querySelector("meta[name='theme-color']")
      ?.setAttribute('content', themeColors[theme])
  }, [theme])

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

        <div className='grid grid-cols-2 gap-3'>
          <ThemeColumn
            icon={<Sun className='size-3.5' />}
            label={t('theme.light')}
            items={LIGHT_THEMES}
            currentTheme={theme}
            onSelect={setTheme}
          />

          <div className='border-l border-border/40 pl-3'>
            <ThemeColumn
              icon={<Moon className='size-3.5' />}
              label={t('theme.dark')}
              items={DARK_THEMES}
              currentTheme={theme}
              onSelect={setTheme}
            />
          </div>
        </div>
      </PopoverContent>
    </Popover>
  )
}