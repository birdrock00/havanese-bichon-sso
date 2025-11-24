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


import { z } from 'zod'
import { useFieldArray, useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { toast } from '@/hooks/use-toast'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  Form,
  FormControl,
  FormDescription,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Textarea } from '@/components/ui/textarea'
import { OAuth2Entity } from '../data/schema'
import { Checkbox } from '@/components/ui/checkbox'
import { cn } from '@/lib/utils'
import { Loader2, MinusCircle, Plus } from 'lucide-react'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { create_oauth2, update_oauth2 } from '@/api/oauth2/api'
import { ToastAction } from '@/components/ui/toast'
import { AxiosError } from 'axios'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import useProxyList from '@/hooks/use-proxy'
import { useTranslation } from 'react-i18next'

const getParamSchema = (t: (key: string) => string) => z.object({
  key: z.string({ required_error: t('oauth2.keyIsRequired') }).min(1, t('oauth2.keyCannotBeEmpty')),
  value: z.string({ required_error: t('oauth2.valueIsRequired') }).min(1, t('oauth2.valueCannotBeEmpty')),
});

const paramSchema = z.object({
  key: z.string({ required_error: 'Key is required' }).min(1, "Key cannot be empty"),
  value: z.string({ required_error: 'Value is required' }).min(1, "Value cannot be empty"),
});

const getScopeSchema = (t: (key: string) => string) => z.object({
  value: z.string({ required_error: t('oauth2.valueIsRequired') }).min(1, t('oauth2.valueCannotBeEmpty')),
});

const scopeSchema = z.object({
  value: z.string({ required_error: 'Value is required' }).min(1, "Value cannot be empty"),
});

const extraparamSchema = z.record(z.string()).optional();
const authorizescopeSchema = z.array(z.string()).optional();


function convertToExtraParamsSchema(
  record: z.infer<typeof extraparamSchema>
): z.infer<typeof paramSchema>[] {
  if (!record) {
    return [];
  }
  return Object.entries(record).map(([key, value]) => ({
    key,
    value,
  }));
}


function convertToScopeSchema(authorizeScopes: z.infer<typeof authorizescopeSchema>): z.infer<typeof scopeSchema>[] {
  if (!authorizeScopes || authorizeScopes.length === 0) {
    return [];
  }

  return authorizeScopes.map((scope) => ({
    value: scope,
  }));
}

const getOAuth2Schema = (t: (key: string) => string) => z.object({
  description: z.string().max(255, { message: t('oauth2.descriptionMustNotExceed255Characters') }).optional(),
  client_id: z.string({
    required_error: t('oauth2.clientIdIsRequired'),
  }).min(1, { message: t('oauth2.clientIdCannotBeEmpty') }),
  client_secret: z.string().optional(),
  auth_url: z.string({
    required_error: t('oauth2.authorizationUrlIsRequired'),
  })
    .min(1, { message: t('oauth2.authorizationUrlCannotBeEmpty') })
    .url({ message: t('oauth2.invalidAuthorizationUrlFormat') }),

  token_url: z.string({
    required_error: t('oauth2.tokenUrlIsRequired'),
  })
    .min(1, { message: t('oauth2.tokenUrlCannotBeEmpty') })
    .url({ message: t('oauth2.invalidTokenUrlFormat') }),

  redirect_uri: z.string({
    required_error: t('oauth2.redirectUriIsRequired'),
  })
    .min(1, { message: t('oauth2.redirectUriCannotBeEmpty') })
    .url({ message: t('oauth2.invalidRedirectUriFormat') }),

  scopes: z.array(getScopeSchema(t)).optional(),
  extra_params: z.array(getParamSchema(t)).optional(),
  enabled: z.boolean(),
  use_proxy: z.number().optional(),
});

const oauth2Schema = z.object({
  description: z.string().max(255, { message: "Description must not exceed 255 characters." }).optional(),
  client_id: z.string({
    required_error: "Client ID is required",
  }).min(1, { message: "Client ID cannot be empty" }),
  client_secret: z.string().optional(),
  auth_url: z.string({
    required_error: "Authorization URL is required",
  })
    .min(1, { message: "Authorization URL cannot be empty" })
    .url({ message: "Invalid Authorization URL format" }),

  token_url: z.string({
    required_error: "Token URL is required",
  })
    .min(1, { message: "Token URL cannot be empty" })
    .url({ message: "Invalid Token URL format" }),

  redirect_uri: z.string({
    required_error: "Redirect URI is required",
  })
    .min(1, { message: "Redirect URI cannot be empty" })
    .url({ message: "Invalid Redirect URI format" }),

  scopes: z.array(scopeSchema).optional(),
  extra_params: z.array(paramSchema).optional(),
  enabled: z.boolean(),
  use_proxy: z.number().optional(),
});

export type OAuth2Form = z.infer<typeof oauth2Schema>;


interface Props {
  currentRow?: OAuth2Entity
  open: boolean
  onOpenChange: (open: boolean) => void
}

const defaultValues = {
  description: undefined,
  client_id: '',
  client_secret: '',
  auth_url: '',
  token_url: '',
  redirect_uri: '',
  extra_params: [],
  scopes: [],
  enabled: true,
  use_proxy: undefined
};


export function ActionDialog({ currentRow, open, onOpenChange }: Props) {
  const { t } = useTranslation()
  const isEdit = !!currentRow
  const form = useForm<OAuth2Form>({
    resolver: zodResolver(getOAuth2Schema(t)),
    defaultValues: isEdit
      ? {
        description: currentRow.description ?? undefined,
        client_id: currentRow.client_id,
        client_secret: undefined,
        auth_url: currentRow.auth_url,
        token_url: currentRow.token_url,
        redirect_uri: currentRow.redirect_uri,
        extra_params: currentRow.extra_params ? convertToExtraParamsSchema(currentRow.extra_params) : undefined,
        scopes: currentRow.scopes ? convertToScopeSchema(currentRow.scopes) : undefined,
        enabled: currentRow.enabled,
        use_proxy: currentRow.use_proxy === null ? undefined : currentRow.use_proxy,
      }
      : defaultValues,
  });

  const { proxyOptions } = useProxyList();

  const { fields: params, append: params_append, remove: params_remove } = useFieldArray({
    name: 'extra_params',
    control: form.control,
  })

  const { fields: scopes, append: scopes_append, remove: scopes_remove } = useFieldArray({
    name: 'scopes',
    control: form.control,
  })


  const queryClient = useQueryClient();

  const createMutation = useMutation({
    mutationFn: create_oauth2,
    onSuccess: handleSuccess,
    onError: handleError
  });

  const updateMutation = useMutation({
    mutationFn: (data: Record<string, any>) => update_oauth2(currentRow?.id!, data),
    onSuccess: handleSuccess,
    onError: handleError
  })

  function handleSuccess() {
    toast({
      title: `OAuth2 ${isEdit ? t('oauth2.updated') : t('oauth2.created')}`,
      description: t('oauth2.yourOAuth2ApplicationHasBeenSuccessfully', { action: isEdit ? t('oauth2.updated').toLowerCase() : t('oauth2.created').toLowerCase() }),
      action: <ToastAction altText={t('common.close')}>{t('common.close')}</ToastAction>,
    });

    queryClient.invalidateQueries({ queryKey: ['oauth2-list'] });
    form.reset();
    onOpenChange(false);
  }
  function handleError(error: AxiosError) {
    const errorMessage = (error.response?.data as { message?: string })?.message ||
      error.message ||
      t('oauth2.updateOrCreationFailed', { action: isEdit ? t('oauth2.updateFailed') : t('oauth2.creationFailed') });

    toast({
      variant: "destructive",
      title: `OAuth2 ${isEdit ? t('oauth2.updateFailed') : t('oauth2.creationFailed')}`,
      description: errorMessage as string,
      action: <ToastAction altText={t('common.tryAgain')}>{t('common.tryAgain')}</ToastAction>,
    });
    console.error(error);
  }

  const onSubmit = (values: OAuth2Form) => {
    if (!isEdit) {
      if (!values.client_secret) {
        form.setError('client_secret', {
          type: 'manual',
          message: t('oauth2.clientSecretIsRequired')
        });
        return;
      }
      if (values.client_secret.length < 1) {
        form.setError('client_secret', {
          type: 'manual',
          message: t('oauth2.clientSecretCannotBeEmpty')
        });
        return;
      }
    }

    const prepareClientSecret = (secret: string | undefined) => {
      return secret && secret.trim() !== '' ? secret : undefined;
    };

    if (isEdit) {
      updateMutation.mutate({
        description: values.description,
        client_id: values.client_id,
        client_secret: prepareClientSecret(values.client_secret),
        auth_url: values.auth_url,
        token_url: values.token_url,
        redirect_uri: values.redirect_uri,
        extra_params: values.extra_params?.reduce((acc, item) => ({ ...acc, [item.key]: item.value }), {}),
        scopes: values.scopes?.map(scope => scope.value),
        enabled: values.enabled,
        use_proxy: values.use_proxy
      });
    } else {
      createMutation.mutate({
        description: values.description,
        client_id: values.client_id,
        client_secret: values.client_secret!,
        auth_url: values.auth_url,
        token_url: values.token_url,
        redirect_uri: values.redirect_uri,
        extra_params: values.extra_params?.reduce((acc, item) => ({ ...acc, [item.key]: item.value }), {}),
        scopes: values.scopes?.map(scope => scope.value),
        enabled: values.enabled,
        use_proxy: values.use_proxy
      });
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(state) => {
        form.reset()
        onOpenChange(state)
      }}
    >
      <DialogContent className='w-full md:max-w-4xl'>
        <DialogHeader className='text-left mb-4'>
          <DialogTitle>{isEdit ? t('oauth2.edit') : t('oauth2.addNew')}</DialogTitle>
          <DialogDescription>
            {isEdit ? t('oauth2.updateHere') : t('oauth2.createNewHere')}
            {t('oauth2.clickSaveWhenDone')}
          </DialogDescription>
        </DialogHeader>
        <div className="flex items-center justify-start gap-2 mb-4">
          <span className="text-sm text-muted-foreground mr-2">
            {t('oauth2.quickPresets')}
          </span>
          <Button
            variant="secondary"
            size="sm"
            onClick={() => {
              form.setValue("auth_url", "https://accounts.google.com/o/oauth2/v2/auth");
              form.setValue("token_url", "https://oauth2.googleapis.com/token");
              form.setValue("enabled", true);
              form.setValue("scopes", [{ value: "https://mail.google.com/" }]);
              form.setValue("extra_params", [{ key: "access_type", value: "offline" }, { key: "prompt", value: "consent" }])
            }}
          >
            {t('oauth2.gmail')}
          </Button>

          <Button
            variant="secondary"
            size="sm"
            onClick={() => {
              form.setValue("auth_url", "https://login.microsoftonline.com/consumers/oauth2/v2.0/authorize");
              form.setValue("token_url", "https://login.microsoftonline.com/consumers/oauth2/v2.0/token");
              form.setValue("enabled", true);
              form.setValue("scopes", [{ value: "https://outlook.office.com/IMAP.AccessAsUser.All" }, { value: "offline_access" }]);
              form.setValue("extra_params", [{ key: "prompt", value: "consent" }])
            }}
          >
            {t('oauth2.outlook')}
          </Button>
        </div>
        <ScrollArea className='h-[28rem] w-full pr-4 -mr-4 py-1'>
          <Form {...form}>
            <form
              id='oauth2-form'
              onSubmit={form.handleSubmit(onSubmit)}
              className='space-y-4 p-0.5'
            >
              <FormField
                control={form.control}
                name='enabled'
                render={({ field }) => (
                  <FormItem className='flex flex-row items-center gap-x-2'>
                    <FormControl>
                      <Checkbox
                        className='mt-2'
                        checked={field.value}
                        onCheckedChange={field.onChange}
                      />
                    </FormControl>
                    <FormLabel>{t('oauth2.enabled')}</FormLabel>
                    <FormDescription>
                      {t('oauth2.whenDisabled')}
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name='client_id'
                render={({ field }) => (
                  <FormItem className='flex flex-col gap-y-1 space-y-0'>
                    <FormLabel className='mb-1'>{t('oauth2.clientIdLabel')}</FormLabel>
                    <FormControl>
                      <Input
                        placeholder={t('oauth2.enterYourClientId')}
                        {...field}
                      />
                    </FormControl>
                    <FormDescription>
                      {t('oauth2.theUniqueIdentifierForYourApplication')}
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name='client_secret'
                render={({ field }) => (
                  <FormItem className='flex flex-col gap-y-1 space-y-0'>
                    <FormLabel className='mb-1'>{t('oauth2.clientSecretLabel')}</FormLabel>
                    <FormControl>
                      <Input
                        placeholder={isEdit ? t('oauth2.leaveEmptyToKeepExistingSecret') : t('oauth2.enterYourClientSecret')}
                        {...field}
                      />
                    </FormControl>
                    <FormDescription>
                      {isEdit
                        ? t('oauth2.leaveEmptyToKeepTheExistingSecret')
                        : t('oauth2.aSecretKeyProvidedByTheOAuthProvider')}
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name='auth_url'
                render={({ field }) => (
                  <FormItem className='flex flex-col gap-y-1 space-y-0'>
                    <FormLabel className='mb-1'>{t('oauth2.authUrlLabel')}</FormLabel>
                    <FormControl>
                      <Input
                        placeholder={t('oauth2.enterTheAuthorizationUrl')}
                        {...field}
                      />
                    </FormControl>
                    <FormDescription>
                      {t('oauth2.theUrlWhereUsersWillBeRedirected')}
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name='token_url'
                render={({ field }) => (
                  <FormItem className='flex flex-col gap-y-1 space-y-0'>
                    <FormLabel className='mb-1'>{t('oauth2.tokenUrlLabel')}</FormLabel>
                    <FormControl>
                      <Input
                        placeholder={t('oauth2.enterTheTokenUrl')}
                        {...field}
                      />
                    </FormControl>
                    <FormDescription>
                      {t('oauth2.theUrlUsedToExchange')}
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name='redirect_uri'
                render={({ field }) => (
                  <FormItem className='flex flex-col gap-y-1 space-y-0'>
                    <FormLabel className='mb-1'>{t('oauth2.redirectUrlLabel')}</FormLabel>
                    <FormControl>
                      <Input
                        placeholder={t('oauth2.enterYourRedirectUrl')}
                        {...field}
                      />
                    </FormControl>
                    <FormDescription>
                      {t('oauth2.theRedirectUrlAfterAuthorization')}
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <div>
                {scopes.map((field, index) => (
                  <div className="flex flex-col gap-4 sm:flex-row sm:items-center" key={field.id + index}>
                    <FormField
                      control={form.control}
                      name={`scopes.${index}.value`}
                      render={({ field }) => (
                        <FormItem className="flex-1">
                          <FormLabel className={cn(index !== 0 && "sr-only")}>{t('oauth2.scope')}</FormLabel>
                          <FormDescription className={cn(index !== 0 && "sr-only")}>
                            {t('oauth2.enterTheScopeHere')}
                          </FormDescription>
                          <FormControl>
                            <Input {...field} />
                          </FormControl>
                          <FormMessage />
                        </FormItem>
                      )}
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => scopes_remove(index)}
                      className={cn(
                        "text-red-500 hover:text-red-700 sm:self-center",
                        index === 0 && "sm:mt-14"
                      )}
                    >
                      <MinusCircle className="h-5 w-5" />
                    </Button>
                  </div>
                ))}
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="mt-2"
                  onClick={() => scopes_append({ value: "" })}
                >
                  <Plus className="mr-2 h-4 w-4" /> {t('oauth2.addScope')}
                </Button>
              </div>
              <div>
                {params.map((field, index) => (
                  <div className="flex flex-col gap-4 sm:flex-row sm:items-center" key={field.id + index}>
                    <div className="flex flex-1 gap-4">
                      <FormField
                        control={form.control}
                        name={`extra_params.${index}.key`}
                        render={({ field }) => (
                          <FormItem className="flex-1">
                            <FormLabel className={cn(index !== 0 && "sr-only")}>{t('oauth2.key')}</FormLabel>
                            <FormDescription className={cn(index !== 0 && "sr-only")}>
                              {t('oauth2.enterTheKeyHere')}
                            </FormDescription>
                            <FormControl>
                              <Input {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                      <FormField
                        control={form.control}
                        name={`extra_params.${index}.value`}
                        render={({ field }) => (
                          <FormItem className="flex-1">
                            <FormLabel className={cn(index !== 0 && "sr-only")}>{t('oauth2.value')}</FormLabel>
                            <FormDescription className={cn(index !== 0 && "sr-only")}>
                              {t('oauth2.enterTheValueHere')}
                            </FormDescription>
                            <FormControl>
                              <Input {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => params_remove(index)}
                      className={cn(
                        "text-red-500 hover:text-red-700 sm:self-center",
                        index === 0 && "sm:mt-14"
                      )}
                    >
                      <MinusCircle className="h-5 w-5" />
                    </Button>
                  </div>
                ))}
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="mt-2"
                  onClick={() => params_append({ key: "", value: "" })}
                >
                  <Plus className="mr-2 h-4 w-4" /> {t('oauth2.addExtraParams')}
                </Button>
              </div>
              <FormField
                control={form.control}
                name='use_proxy'
                render={({ field }) => (
                  <FormItem>
                    <FormLabel className="flex items-center justify-between">{t('oauth2.useProxyOptional')}</FormLabel>
                    <FormControl>
                      <Select
                        onValueChange={(val) => field.onChange(Number(val))}
                        defaultValue={field.value?.toString()}
                      >
                        <FormControl>
                          <SelectTrigger>
                            <SelectValue placeholder={t('oauth2.selectAProxy')} />
                          </SelectTrigger>
                        </FormControl>
                        <SelectContent>
                          {proxyOptions && proxyOptions.length > 0 ? (
                            proxyOptions.map((option) => (
                              <SelectItem key={option.value} value={option.value.toString()}>
                                {option.label}
                              </SelectItem>
                            ))
                          ) : (
                            <SelectItem disabled value="__none__">{t('oauth2.noProxyAvailable')}</SelectItem>
                          )}
                        </SelectContent>
                      </Select>
                    </FormControl>
                    <FormDescription className='flex-1'>
                      {t('oauth2.useSocks5ProxyForOAuthRequests')}
                    </FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name='description'
                render={({ field }) => (
                  <FormItem className='flex flex-col gap-y-1 space-y-0'>
                    <FormLabel className='mb-1'>{t('oauth2.descriptionLabel')}</FormLabel>
                    <FormControl>
                      <Textarea
                        placeholder={t('oauth2.describeThePurposeOfTheOauth2Application')}
                        {...field}
                        className="max-h-[240px] min-h-[80px]"
                      />
                    </FormControl>
                    <FormDescription>{t('oauth2.optional')}</FormDescription>
                    <FormMessage />
                  </FormItem>
                )}
              />
            </form>
          </Form>
        </ScrollArea>
        <DialogFooter>
          <Button
            type="submit"
            form="oauth2-form"
            disabled={isEdit ? updateMutation.isPending : createMutation.isPending}
            className="relative"
          >
            {isEdit ? (
              updateMutation.isPending ? (
                <span className="flex items-center justify-center">
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t('oauth2.saving')}
                </span>
              ) : (
                t('oauth2.saveChanges')
              )
            ) : (
              createMutation.isPending ? (
                <span className="flex items-center justify-center">
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t('oauth2.creating')}
                </span>
              ) : (
                t('oauth2.save')
              )
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
