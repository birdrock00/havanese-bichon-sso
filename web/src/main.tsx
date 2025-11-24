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


import { StrictMode } from 'react'
import ReactDOM from 'react-dom/client'
import { AxiosError } from 'axios'
import {
  QueryCache,
  QueryClient,
  QueryClientProvider,
} from '@tanstack/react-query'
import { RouterProvider, createRouter } from '@tanstack/react-router'
import { resetAccessToken } from '@/stores/authStore'
import { toast } from '@/hooks/use-toast'
import { ThemeProvider } from './context/theme-context'
import './index.css'
import './i18n'
// Generated Routes
import { routeTree } from './routeTree.gen'
import { ToastAction } from './components/ui/toast'
import i18n from './i18n'


const handleAxiosError = (error: any) => {
  if (!(error instanceof AxiosError)) return;

  switch (error.response?.status) {
    case 401:
      toast({
        variant: 'destructive',
        title: i18n.t('auth.sessionExpired'),
        description: i18n.t('auth.sessionExpiredDesc'),
        action: <ToastAction altText={i18n.t('common.close')}>{i18n.t('common.close')}</ToastAction>,
      });
      resetAccessToken();
      const currentPath = router.history.location.pathname;
      if (currentPath !== '/sign-in') {
        const redirect = `${router.history.location.href}`;
        router.navigate({ to: '/sign-in', search: { redirect } });
      }
      break;
    case 403:
      router.navigate({ to: '/403' });
      break;
    case 500:
      toast({
        variant: 'destructive',
        title: i18n.t('errors.internalServerError'),
      });
      router.navigate({ to: '/500' });
      break;
    case 304:
      toast({
        variant: 'destructive',
        title: i18n.t('errors.contentNotModified'),
      });
      break;
    default:
      if (error.code === "ERR_NETWORK") {
        toast({
          variant: "destructive",
          title: i18n.t('errors.networkError'),
          description: i18n.t('errors.networkErrorDesc'),
          action: <ToastAction altText={i18n.t('common.tryAgain')}>{i18n.t('common.tryAgain')}</ToastAction>,
        });
      }
  }
};



const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: (failureCount, error) => {
        // eslint-disable-next-line no-console
        if (import.meta.env.DEV) console.log({ failureCount, error })

        if (failureCount >= 0 && import.meta.env.DEV) return false
        if (failureCount > 3 && import.meta.env.PROD) return false

        return !(
          error instanceof AxiosError &&
          [401, 403].includes(error.response?.status ?? 0)
        )
      },
      refetchOnWindowFocus: import.meta.env.PROD,
      staleTime: 10 * 1000, // 10s
    },
    mutations: {
      onError: (error) => {
        handleAxiosError(error)
      },
    },
  },
  queryCache: new QueryCache({
    onError: (error) => {
      handleAxiosError(error)
    },
  }),
})

// Create a new router instance
const router = createRouter({
  routeTree,
  context: { queryClient },
  defaultPreload: 'intent',
  defaultPreloadStaleTime: 0,
})

// Register the router instance for type safety
declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

// Render the app
const rootElement = document.getElementById('root')!
if (!rootElement.innerHTML) {
  const root = ReactDOM.createRoot(rootElement)
  root.render(
    <StrictMode>
      <QueryClientProvider client={queryClient}>
        <ThemeProvider defaultTheme='light' storageKey='vite-ui-theme'>
          <RouterProvider router={router} />
        </ThemeProvider>
      </QueryClientProvider>
    </StrictMode>
  )
}
