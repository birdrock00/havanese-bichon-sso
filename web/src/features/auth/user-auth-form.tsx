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


import {
  HTMLAttributes,
  forwardRef,
  useEffect,
  useMemo,
  useState,
  type ComponentProps,
  type ReactNode,
} from 'react'
import { useForm } from 'react-hook-form'
import { zodResolver } from '@hookform/resolvers/zod'
import { cn, toSearchParams } from '@/lib/utils'
import { getFormSchema, type LoginFormValues } from './schema'
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from '@/components/ui/form'
import { Input } from '@/components/ui/input'
import { PasswordInput } from '@/components/password-input'
import { OtpInput } from '@/components/otp-input'
import { useMutation, useQuery } from '@tanstack/react-query'
import { setToken } from '@/stores/authStore'
import { toast } from '@/hooks/use-toast'
import { AxiosError } from 'axios'
import { ToastAction } from '@/components/ui/toast'
import { useLocation, useNavigate } from '@tanstack/react-router'
import { Button } from '@/components/button'
import { useTranslation } from 'react-i18next'
import i18n from '@/i18n'

/** Plain-language text for the OIDC error codes the server redirects with.
 *  Unknown codes fall back to showing the raw code. */
const OIDC_ERROR_KEYS: Record<string, string> = {
  license_required: 'auth.ssoErrLicense',
  email_domain_not_allowed: 'auth.ssoErrDomain',
  discovery_invalid_issuer: 'settings.sso.errInvalidIssuer',
  discovery_dns: 'settings.sso.errDns',
  discovery_connection_refused: 'settings.sso.errRefused',
  discovery_timeout: 'settings.sso.errTimeout',
  discovery_tls: 'settings.sso.errTls',
  discovery_network: 'settings.sso.errNetwork',
  discovery_redirect: 'settings.sso.errRedirect',
  discovery_not_found: 'settings.sso.errNotFound',
  discovery_access_denied: 'settings.sso.errAccessDenied',
  discovery_server_error: 'settings.sso.errServerError',
  discovery_http_error: 'settings.sso.errServerError',
  discovery_not_json: 'settings.sso.errNotJson',
  discovery_incomplete: 'settings.sso.errIncomplete',
  discovery_issuer_mismatch: 'settings.sso.errIssuerMismatch',
}
import { KeyRound, Loader2, Lock, LogIn, Shield, User } from 'lucide-react'
import { ldapLogin, login, mfaVerify, type LoginResult } from '@/api/users/api'
import { get_sso_status } from '@/api/sso/api'
import { useTheme } from '@/context/theme-context'
import { isTheme } from '@/lib/theme'
import { useEdition } from '@/hooks/use-edition'

type UserAuthFormProps = HTMLAttributes<HTMLDivElement>

/** Fork SSO entry point: the community OIDC login endpoint, carrying the
 *  post-login redirect and honouring an injected base path. */
function buildOidcLoginUrl(redirectTo: string): string {
  const injectedBase = (window as unknown as { __BICHON_BASE__?: string }).__BICHON_BASE__
  const base = !injectedBase || injectedBase === '/' ? '' : injectedBase.replace(/\/$/, '')
  const params = new URLSearchParams()
  if (redirectTo && redirectTo !== '/') {
    params.set('redirect_to', redirectTo)
  }
  const qs = params.toString()
  return `${base}/api/auth/oidc/login${qs ? `?${qs}` : ''}`
}

/** Input with a leading icon; the ref goes to the input so FormControl can
 *  still wire the label and the field. */
const IconField = forwardRef<
  HTMLInputElement,
  ComponentProps<typeof Input> & { icon: ReactNode }
>(({ icon, className, ...props }, ref) => (
  <div className='relative'>
    <span className='pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground'>
      {icon}
    </span>
    <Input ref={ref} className={cn('pl-9', className)} {...props} />
  </div>
))
IconField.displayName = 'IconField'

/** The password input already owns a ref for react-hook-form; the lock icon
 *  sits above its wrapper and the inner input gets the left padding. */
const PasswordField = forwardRef<
  HTMLInputElement,
  ComponentProps<typeof PasswordInput>
>(({ className, ...props }, ref) => (
  <div className='relative'>
    <Lock
      size={16}
      className='pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 text-muted-foreground'
    />
    <PasswordInput
      ref={ref}
      className={cn('[&_input]:pl-9', className)}
      {...props}
    />
  </div>
))
PasswordField.displayName = 'PasswordField'

export function UserAuthForm({ className, ...props }: UserAuthFormProps) {
  const [isLoading, setIsLoading] = useState(false)
  const [mfaChallenge, setMfaChallenge] = useState<string | null>(null)
  const { setTheme } = useTheme();
  const navigate = useNavigate()
  const { t } = useTranslation()
  const { search } = useLocation();
  // The fork's three sign-in params (`redirect`, `local`, `sso_error`) are not
  // declared in a route `validateSearch`, and the router hands the component an
  // empty search object on an initial hard load in that case. Every path that
  // matters IS a hard load: the OIDC callback and the server's error bounce are
  // full-page redirects, and `?local=1` / `?redirect=/x` are typed or linked
  // from outside the app. Merge the raw query in as a fallback so those loads
  // behave like in-app navigation (which still wins, since `search` has the
  // params then). Same source upstream already uses for `oidc_error`.
  const searchParams = useMemo(() => {
    const params = toSearchParams(search)
    const raw = new URLSearchParams(window.location.search)
    for (const key of ['redirect', 'local', 'sso_error'] as const) {
      if (!params.has(key) && raw.has(key)) params.set(key, raw.get(key) ?? '')
    }
    return params
  }, [search])
  const redirect = searchParams.get('redirect') || '/';
  const localOnly = searchParams.get('local') === '1';
  const ssoError = searchParams.get('sso_error');

  // `ssoEnabled` folds in the fork's community `oidc_enabled`; `isPro` keeps
  // upstream's Pro/Enterprise SSO button behaviour.
  const { isPro, oidcEnabled, oidcAutoRedirect, ssoEnabled, ldapEnabled } =
    useEdition()

  // Ask whether the configured provider is actually reachable. A down IdP
  // used to be visible only in the server log: the page offered the button
  // and sent the user to a raw error page. The server caches the probe, so
  // this is one cheap request per page load.
  //
  // Fork note: GET api/auth/oidc/status is only compiled into the Pro server
  // (neither this fork nor public upstream ships the route). On the fork's
  // community OIDC the request falls through to the SPA fallback and answers
  // 200 with index.html, so `available` is undefined and the button would be
  // disabled on every login. Probe only where the answer is real, and never
  // let a missing answer block the fork's env-configured SSO.
  const { data: ssoStatus } = useQuery({
    queryKey: ['sso-status'],
    queryFn: get_sso_status,
    enabled: isPro && ssoEnabled,
    staleTime: 30_000,
    retry: 0,
  })
  const ssoUnavailable =
    isPro && ssoStatus !== undefined && !ssoStatus.available

  // The OIDC callback redirects back here with ?oidc_error=<code>. Surface it
  // once and strip it from the URL so a refresh does not repeat the toast.
  useEffect(() => {
    const params = new URLSearchParams(window.location.search)
    const code = params.get('oidc_error')
    if (!code) return
    const friendly = OIDC_ERROR_KEYS[code]
    toast({
      variant: 'destructive',
      title: t('auth.ssoFailed', 'Single sign-on failed'),
      description: friendly
        ? t(friendly, { status: '-' })
        : t(
            'auth.ssoFailedDesc',
            'The identity provider returned: {{code}}',
            { code },
          ),
    })
    params.delete('oidc_error')
    const query = params.toString()
    window.history.replaceState(
      null,
      '',
      `${window.location.pathname}${query ? `?${query}` : ''}${window.location.hash}`,
    )
  }, [t])

  // The fork's callback and the server bounce failures back with ?sso_error=.
  useEffect(() => {
    if (ssoError) {
      toast({
        variant: 'destructive',
        title: t('auth.loginFailed'),
        description: ssoError,
      })
    }
  }, [ssoError, t])

  // Fork behaviour: when the server is configured for automatic SSO, go
  // straight to the identity provider unless the user asked for the local
  // form (?local=1) or a previous attempt failed.
  useEffect(() => {
    if (oidcEnabled && oidcAutoRedirect && !localOnly && !ssoError) {
      window.location.href = buildOidcLoginUrl(redirect)
    }
  }, [oidcEnabled, oidcAutoRedirect, localOnly, ssoError, redirect])

  // Enterprise LDAP: the form authenticates against the directory (default
  // when enabled); a link lets the user fall back to their local account.
  const [loginMode, setLoginMode] = useState<'local' | 'ldap'>(
    ldapEnabled ? 'ldap' : 'local',
  )

  const formSchema = getFormSchema(t)
  const form = useForm<LoginFormValues>({
    resolver: zodResolver(formSchema),
    defaultValues: {
      username: '',
      password: '',
      code: '',
    },
  })

  const mutation = useMutation({
    mutationFn: (data: LoginFormValues) => {
      if (mfaChallenge) {
        return mfaVerify(mfaChallenge, data.code ?? '')
      }
      if (loginMode === 'ldap') {
        return ldapLogin({
          username: data.username ?? '',
          password: data.password ?? '',
        })
      }
      return login(data)
    },
    retry: 0,
  });

  function handleLoginSuccess(result: LoginResult) {
    // Two-factor step: the server asks for a TOTP code before issuing a token.
    if (result.mfa_required && result.mfa_challenge) {
      setMfaChallenge(result.mfa_challenge)
      return
    }

    if (result.success && result.access_token) {
      setToken(result);

      if (isTheme(result.theme)) {
        setTheme(result.theme);
      }

      if (result.language) {
        i18n.changeLanguage(result.language);
      }

      navigate({ to: redirect });
    }
  }

  function runSubmit(data: LoginFormValues) {
    if (isLoading) return
    setIsLoading(true)

    mutation.mutate(data, {
      onSuccess: (result) => {
        if (!result.success) {
          toast({
            variant: "destructive",
            title: t('auth.loginFailed'),
            description: `${result.error_message!}`,
            action: <ToastAction altText={t('common.tryAgain')}>{t('common.tryAgain')}</ToastAction>,
          })
          setIsLoading(false);
          return
        }
        handleLoginSuccess(result)
        setIsLoading(false);
      },
      onError: (error) => {
        const { t } = i18n
        if (error instanceof AxiosError && error.response && error.response.status === 401) {
          toast({
            variant: "destructive",
            title: t('auth.loginFailed'),
            description: t('auth.invalidPassword'),
            action: <ToastAction altText={t('common.tryAgain')}>{t('common.tryAgain')}</ToastAction>,
          })
        } else {
          toast({
            variant: "destructive",
            title: t('auth.somethingWentWrong'),
            description: (error as Error).message,
            action: <ToastAction altText={t('common.tryAgain')}>{t('common.tryAgain')}</ToastAction>,
          })
        }
        setIsLoading(false)
      }
    });
  }

  async function onSubmit(data: LoginFormValues) {
    if (mfaChallenge && (!data.code || data.code.trim().length !== 6)) {
      toast({
        variant: "destructive",
        title: t('auth.mfaCodeRequired', 'Please enter the 6-digit code from your authenticator app.'),
      })
      return
    }

    runSubmit(data)
  }

  // The sixth digit verifies immediately; the button stays as the fallback for
  // editing an already-complete code and submitting it again.
  function submitCode(code: string) {
    if (code.length !== 6 || isLoading) return
    runSubmit({ ...form.getValues(), code })
  }

  return (
    <div className={cn('grid gap-6', className)} {...props}>
      <Form {...form}>
        {/* The TOTP step stays outside any <form>: with the sign-in fields
            gone, a password manager would otherwise read this lone field as
            part of the login form and fill it on load. */}
        {mfaChallenge ? (
          <div className='grid gap-5'>
            <p className='text-sm text-muted-foreground'>
              {t('auth.mfaLoginDesc', 'Enter the 6-digit code shown in your authenticator app to finish signing in.')}
            </p>
            <FormField
              control={form.control}
              name='code'
              render={({ field }) => (
                <FormItem className='space-y-2'>
                  <FormLabel>{t('auth.mfaCodePlaceholder', 'Authentication code')}</FormLabel>
                  <FormControl>
                    <OtpInput
                      value={field.value ?? ''}
                      onChange={(value) => {
                        field.onChange(value)
                        if (value.length === 6) submitCode(value)
                      }}
                      autoFocus
                      className='justify-center py-1'
                    />
                  </FormControl>
                  <FormMessage />
                </FormItem>
              )}
            />
            <Button
              type='button'
              size='lg'
              className='mt-1 w-full'
              disabled={isLoading}
              onClick={() => form.handleSubmit(onSubmit)()}
            >
              {isLoading ? <Loader2 className='animate-spin' /> : <KeyRound size={16} className='mr-2' />}
              {t('auth.mfaVerify', 'Verify')}
            </Button>
          </div>
        ) : (
          <form onSubmit={form.handleSubmit(onSubmit)}>
            <div className='grid gap-5'>
              <FormField
                control={form.control}
                name='username'
                render={({ field }) => (
                  <FormItem className='space-y-2'>
                    <FormLabel>{t('auth.username')}</FormLabel>
                    <FormControl>
                      <IconField
                        icon={<User size={16} />}
                        autoFocus
                        autoComplete='username'
                        {...field}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                control={form.control}
                name='password'
                render={({ field }) => (
                  <FormItem className='space-y-2'>
                    <FormLabel>{t('auth.password')}</FormLabel>
                    <FormControl>
                      <PasswordField
                        autoComplete='current-password'
                        placeholder='********'
                        {...field}
                      />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <Button size='lg' className='mt-1 w-full' disabled={isLoading}>
                {isLoading ? <Loader2 className='animate-spin' /> : <LogIn size={16} className='mr-2' />}
                {t('auth.login')}
              </Button>

              {(isPro || oidcEnabled) && (
                <>
                  <Button
                    variant='outline'
                    size='lg'
                    className='w-full'
                    type='button'
                    onClick={() => {
                      if (!ssoEnabled) {
                        toast({
                          title: t('auth.ssoNotEnabled'),
                          description: t('auth.ssoNotEnabledDesc'),
                        })
                      } else if (ssoUnavailable) {
                        // Fresh probe says the provider is down. Navigating
                        // would just hang and bounce back; tell the user
                        // now and point them at the local login above.
                        toast({
                          variant: 'destructive',
                          title: t('auth.ssoUnavailable'),
                        })
                      } else {
                        window.location.href = buildOidcLoginUrl(redirect)
                      }
                    }}
                  >
                    <Shield size={16} className='mr-2' />
                    {t('auth.ssoLogin')}
                  </Button>
                  {ssoUnavailable && (
                    <p className='text-center text-xs font-medium text-destructive'>
                      {t('auth.ssoUnavailable')}
                    </p>
                  )}
                </>
              )}

              {/* `ldapEnabled` already means "Enterprise + flag set" — the
                  server folds the license into it so the form can never
                  offer a mode the login endpoint would refuse. */}
              {ldapEnabled && (
                <Button
                  variant='link'
                  className='w-full'
                  type='button'
                  onClick={() => setLoginMode((m) => (m === 'ldap' ? 'local' : 'ldap'))}
                >
                  {loginMode === 'ldap'
                    ? t('auth.localLogin', 'Sign in with your local account')
                    : t('auth.ldapLogin', 'Sign in with LDAP')}
                </Button>
              )}
            </div>
          </form>
        )}
      </Form>
    </div>
  )
}
