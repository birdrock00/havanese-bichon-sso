//
// Fork coverage for community OIDC single sign-on.
//
// Upstream's own sign-in tests mock `isPro: true`, so the path this fork
// actually runs (community edition + BICHON_OIDC_ENABLED) had no coverage —
// which is how the availability probe was able to disable the SSO button in
// the 2.4.3 merge, and how the hard-load param loss stayed invisible.
//
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { render, screen } from '@/test/test-utils'
import i18n from '@/i18n'
import { UserAuthForm } from '../user-auth-form'

const toastMock = vi.hoisted(() => vi.fn())
const probeCalls = vi.hoisted(() => ({ n: 0 }))
const edition = vi.hoisted(() => ({
  isPro: false,
  oidcEnabled: true,
  oidcAutoRedirect: false,
  ssoEnabled: true,
  ldapEnabled: false,
}))

// No route `validateSearch` on the sign-in route, so the router gives the
// component an empty search object; the component must fall back to the real
// URL. Mocking `search: {}` reproduces a hard load.
vi.mock('@tanstack/react-router', () => ({
  useNavigate: () => () => {},
  useLocation: () => ({ search: {} }),
}))

vi.mock('@/hooks/use-edition', () => ({ useEdition: () => edition }))

vi.mock('@/context/theme-context', () => ({
  useTheme: () => ({ setTheme: () => {} }),
}))

vi.mock('@/api/users/api', () => ({
  login: vi.fn(),
  ldapLogin: vi.fn(),
  mfaVerify: vi.fn(),
}))

vi.mock('@/hooks/use-toast', () => ({
  toast: (...args: unknown[]) => toastMock(...args),
}))

vi.mock('@/api/sso/api', () => ({
  get_sso_status: vi.fn(async () => {
    probeCalls.n += 1
    // What a Pro-only endpoint would answer; on the fork this route does not
    // exist at all and the SPA fallback answers HTML instead.
    return { configured: true, available: false, code: 'discovery_timeout', checked_at: 1 }
  }),
}))

const setLocation = (search: string) => {
  vi.stubGlobal('location', { search, href: `https://bichon.test/sign-in${search}` })
}

beforeEach(() => {
  probeCalls.n = 0
  toastMock.mockClear()
  edition.isPro = false
  edition.oidcEnabled = true
  edition.oidcAutoRedirect = false
  edition.ssoEnabled = true
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('fork community OIDC sign-in', () => {
  it('offers SSO and never probes the Pro-only status endpoint', async () => {
    setLocation('')
    render(<UserAuthForm />)

    expect(await screen.findByText(i18n.t('auth.ssoLogin'))).toBeInTheDocument()
    expect(
      screen.queryByText(i18n.t('auth.ssoUnavailable')),
    ).not.toBeInTheDocument()
    // The probe is what broke 2.4.3: it must not run where the answer is fake.
    expect(probeCalls.n).toBe(0)
  })

  it('shows the SSO failure the server bounced back on a hard load', async () => {
    setLocation('?sso_error=provider+rejected+the+login')
    render(<UserAuthForm />)

    await vi.waitFor(() => {
      expect(toastMock).toHaveBeenCalledWith(
        expect.objectContaining({ description: 'provider rejected the login' }),
      )
    })
  })

  it('sends the SSO button to the fork endpoint with redirect and base path', async () => {
    setLocation('?redirect=%2Fsettings')
    render(<UserAuthForm />)

    const button = await screen.findByText(i18n.t('auth.ssoLogin'))
    ;(button.closest('button') ?? button).click()

    const href = (globalThis as unknown as { location: { href: string } }).location.href
    expect(href).toContain('/api/auth/oidc/login')
    expect(href).toContain('redirect_to=%2Fsettings')
  })
})
