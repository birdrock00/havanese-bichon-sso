import { z } from 'zod'

const paramEntry = (t: (key: string) => string) =>
  z.object({
    key: z
      .string({ required_error: t('oauth2.keyIsRequired') })
      .min(1, t('oauth2.keyCannotBeEmpty')),
    value: z
      .string({ required_error: t('oauth2.valueIsRequired') })
      .min(1, t('oauth2.valueCannotBeEmpty')),
  })

const scopeEntry = (t: (key: string) => string) =>
  z.object({
    value: z
      .string({ required_error: t('oauth2.valueIsRequired') })
      .min(1, t('oauth2.valueCannotBeEmpty')),
  })

export const getOAuth2Schema = (t: (key: string) => string) =>
  z.object({
    description: z
      .string()
      .max(255, { message: t('oauth2.descriptionMustNotExceed255Characters') })
      .optional(),
    client_id: z
      .string({
        required_error: t('oauth2.clientIdIsRequired'),
      })
      .min(1, { message: t('oauth2.clientIdCannotBeEmpty') }),
    client_secret: z.string().optional(),
    auth_url: z
      .string({
        required_error: t('oauth2.authorizationUrlIsRequired'),
      })
      .min(1, { message: t('oauth2.authorizationUrlCannotBeEmpty') })
      .url({ message: t('oauth2.invalidAuthorizationUrlFormat') }),
    token_url: z
      .string({
        required_error: t('oauth2.tokenUrlIsRequired'),
      })
      .min(1, { message: t('oauth2.tokenUrlCannotBeEmpty') })
      .url({ message: t('oauth2.invalidTokenUrlFormat') }),
    redirect_uri: z
      .string({
        required_error: t('oauth2.redirectUriIsRequired'),
      })
      .min(1, { message: t('oauth2.redirectUriCannotBeEmpty') })
      .url({ message: t('oauth2.invalidRedirectUriFormat') }),
    scopes: z.array(scopeEntry(t)).optional(),
    extra_params: z.array(paramEntry(t)).optional(),
    enabled: z.boolean(),
    use_proxy: z.number().optional(),
  })

export type OAuth2FormValues = z.infer<ReturnType<typeof getOAuth2Schema>>
