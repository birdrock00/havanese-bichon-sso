import { z } from 'zod'
import { useForm } from 'react-hook-form'
import { CaretSortIcon, CheckIcon } from '@radix-ui/react-icons'
import { zodResolver } from '@hookform/resolvers/zod'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import {
    Form,
    FormControl,
    FormDescription,
    FormField,
    FormItem,
    FormLabel,
    FormMessage,
} from '@/components/ui/form'
import { RadioGroup } from '@/components/ui/radio-group'
import { useTheme, Theme } from '@/context/theme-context'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { Command, CommandEmpty, CommandGroup, CommandInput, CommandItem, CommandList } from '@/components/ui/command'
import { useTranslation } from 'react-i18next'
import { useCurrentUser } from '@/hooks/use-current-user'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { update_user } from '@/api/users/api'
import { toast } from '@/hooks/use-toast'
import { AxiosError } from 'axios'
import { Card } from '@/components/ui/card'

const languages = [
    { value: 'ar', label: 'العربية' },
    { value: 'da', label: 'Dansk' },
    { value: 'de', label: 'Deutsch' },
    { value: 'en', label: 'English' },
    { value: 'es', label: 'Español' },
    { value: 'fi', label: 'Suomi' },
    { value: 'fr', label: 'Français' },
    { value: 'it', label: 'Italiano' },
    { value: 'jp', label: '日本語' },
    { value: 'ko', label: '한국어' },
    { value: 'nl', label: 'Nederlands' },
    { value: 'no', label: 'Norsk' },
    { value: 'pl', label: 'Polski' },
    { value: 'pt', label: 'Português' },
    { value: 'ru', label: 'Русский' },
    { value: 'sv', label: 'Svenska' },
    { value: 'zh', label: '中文' },
    { value: 'zh-tw', label: '繁體中文' },
]

const LIGHT_THEMES = [
    { value: 'light', labelKey: 'theme.default', background: '#ffffff', color: '#0f172a', muted: '#cbd5e1' },
    { value: 'rose-light', labelKey: 'theme.rose', background: '#fff1f2', color: '#e11d48', muted: '#fda4af' },
    { value: 'orange-light', labelKey: 'theme.orange', background: '#fff7ed', color: '#ea580c', muted: '#fdba74' },
    { value: 'green-light', labelKey: 'theme.green', background: '#f0fdf4', color: '#16a34a', muted: '#86efac' },
    { value: 'yellow-light', labelKey: 'theme.yellow', background: '#fefce8', color: '#ca8a04', muted: '#fde047' },
    { value: 'blue-light', labelKey: 'theme.blue', background: '#eff6ff', color: '#2563eb', muted: '#bfdbfe' },
] as const

const DARK_THEMES = [
    { value: 'dark', labelKey: 'theme.default', background: '#020817', color: '#94a3b8', muted: '#334155' },
    { value: 'rose-dark', labelKey: 'theme.rose', background: '#1f1115', color: '#e11d48', muted: '#881337' },
    { value: 'orange-dark', labelKey: 'theme.orange', background: '#1c120d', color: '#f97316', muted: '#9a3412' },
    { value: 'green-dark', labelKey: 'theme.green', background: '#0d1b12', color: '#22c55e', muted: '#166534' },
    { value: 'yellow-dark', labelKey: 'theme.yellow', background: '#1c1608', color: '#facc15', muted: '#a16207' },
    { value: 'blue-dark', labelKey: 'theme.blue', background: '#172554', color: '#60a5fa', muted: '#1e3a8a' }
] as const

const ALL_THEME_VALUES = [...LIGHT_THEMES.map(t => t.value), ...DARK_THEMES.map(t => t.value)] as [string, ...string[]]

const appearanceSchema = (t: (key: string) => string) => z.object({
    theme: z.enum(ALL_THEME_VALUES, {
        required_error: t('settings.appearance.validation.theme.required'),
    }),
    language: z.string({
        required_error: t('settings.appearance.validation.language.required'),
    })
})

type AppearanceFormValues = z.infer<ReturnType<typeof appearanceSchema>>

function ThemePreview({ background, primary, muted }: { background: string; primary: string; muted: string }) {
    return (
        <div className='flex h-12 w-full flex-col gap-1.5 rounded-md border p-2 shadow-sm' style={{ backgroundColor: background, borderColor: 'transparent' }}>
            <div className='flex gap-1'>
                <span className='h-1.5 w-1.5 rounded-full' style={{ backgroundColor: muted }} />
                <span className='h-1.5 w-1.5 rounded-full' style={{ backgroundColor: primary }} />
            </div>
            <div className='h-1.5 w-3/4 rounded-sm' style={{ backgroundColor: primary }} />
            <div className='h-1.5 w-1/2 rounded-sm opacity-50' style={{ backgroundColor: muted }} />
        </div>
    )
}

export function AppearanceForm() {
    const { data: user } = useCurrentUser()
    const queryClient = useQueryClient()
    const { t, i18n } = useTranslation()
    const { theme, setTheme } = useTheme()

    const form = useForm<AppearanceFormValues>({
        resolver: zodResolver(appearanceSchema(t)),
        mode: 'onChange',
        defaultValues: {
            theme: (theme as any) || 'light',
            language: i18n.language || 'en',
        },
    })

    const mutation = useMutation({
        mutationFn: async (values: AppearanceFormValues) => {
            return update_user(user!.id, values)
        },
        onSuccess: () => {
            queryClient.invalidateQueries({ queryKey: ['current-user'] })
            toast({ title: t('settings.profile.toast.updated') })
        },
        onError: (err: AxiosError) => {
            toast({
                variant: 'destructive',
                title: t('settings.profile.toast.update_failed'),
                description: (err.response?.data as any)?.message || err.message,
            })
        },
    })

    function onSubmit(data: AppearanceFormValues) {
        i18n.changeLanguage(data.language)
        setTheme(data.theme as Theme)
        mutation.mutate(data)
    }

    return (
        <div className="w-full max-w-6xl ml-0 px-4">
            <Form {...form}>
                <form onSubmit={form.handleSubmit(onSubmit)} className="space-y-10 w-full max-w-screen-xl mx-auto px-4 md:px-6">
                    <FormField
                        control={form.control}
                        name='language'
                        render={({ field }) => (
                            <FormItem className='flex flex-col'>
                                <FormLabel>{t('settings.appearance.field.language')}</FormLabel>
                                <Popover>
                                    <PopoverTrigger asChild>
                                        <FormControl>
                                            <Button
                                                variant='outline'
                                                role='combobox'
                                                className={cn(
                                                    'w-[400px] justify-between',
                                                    !field.value && 'text-muted-foreground'
                                                )}
                                            >
                                                {field.value
                                                    ? languages.find((l) => l.value === field.value)?.label
                                                    : t('settings.appearance.placeholder.select_language')}
                                                <CaretSortIcon className='ms-2 h-4 w-4 shrink-0 opacity-50' />
                                            </Button>
                                        </FormControl>
                                    </PopoverTrigger>
                                    <PopoverContent className='w-[400px] p-0' align="start">
                                        <Command>
                                            <CommandInput placeholder={t('settings.appearance.command.search')} />
                                            <CommandEmpty>{t('settings.appearance.command.no_results')}</CommandEmpty>
                                            <CommandList>
                                                <CommandGroup>
                                                    {languages.map((language) => (
                                                        <CommandItem
                                                            value={language.label}
                                                            key={language.value}
                                                            onSelect={() => {
                                                                form.setValue('language', language.value)
                                                            }}
                                                        >
                                                            <CheckIcon
                                                                className={cn(
                                                                    'mr-2 h-4 w-4',
                                                                    language.value === field.value ? 'opacity-100' : 'opacity-0'
                                                                )}
                                                            />
                                                            {language.label}
                                                        </CommandItem>
                                                    ))}
                                                </CommandGroup>
                                            </CommandList>
                                        </Command>
                                    </PopoverContent>
                                </Popover>
                                <FormDescription>
                                    {t('settings.appearance.description.language')}
                                </FormDescription>
                                <FormMessage />
                            </FormItem>
                        )}
                    />

                    <FormField
                        control={form.control}
                        name='theme'
                        render={({ field }) => (
                            <FormItem className="space-y-4">
                                <div>
                                    <FormLabel className="text-base">{t('settings.appearance.field.theme')}</FormLabel>
                                    <FormDescription>{t('settings.appearance.description.theme')}</FormDescription>
                                </div>
                                <RadioGroup
                                    onValueChange={(v) => {
                                        setTheme(v as Theme);
                                        field.onChange(v)
                                    }}
                                    defaultValue={field.value}
                                    className='grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-4 pt-2'
                                >
                                    {[...LIGHT_THEMES, ...DARK_THEMES].map((item) => {
                                        const selected = field.value === item.value

                                        return (
                                            <Card
                                                key={item.value}
                                                onClick={() => {
                                                    setTheme(item.value as Theme)
                                                    field.onChange(item.value)
                                                }}
                                                className={cn(
                                                    "cursor-pointer overflow-hidden border-2 transition-all hover:shadow-md",
                                                    selected ? "border-primary shadow-md" : "border-muted opacity-80"
                                                )}
                                            >
                                                <div className="p-2">
                                                    <ThemePreview
                                                        background={item.background}
                                                        primary={item.color}
                                                        muted={item.muted}
                                                    />

                                                    <div className="mt-2 flex items-center justify-between">
                                                        <span className="text-[11px] font-medium uppercase truncate">
                                                            {t(item.labelKey)}
                                                        </span>

                                                        {selected && (
                                                            <CheckIcon className="h-3.5 w-3.5 text-primary" />
                                                        )}
                                                    </div>
                                                </div>
                                            </Card>
                                        )
                                    })}
                                </RadioGroup>
                                <FormMessage />
                            </FormItem>
                        )}
                    />

                    <div className="flex justify-start pt-4">
                        <Button type='submit'>
                            {t('settings.appearance.button.update')}
                        </Button>
                    </div>
                </form>
            </Form>
        </div>
    )
}