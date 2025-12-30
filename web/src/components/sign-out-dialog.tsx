import { useNavigate, useLocation } from '@tanstack/react-router'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { resetToken } from '@/stores/authStore'
import { useTranslation } from 'react-i18next'

interface SignOutDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export function SignOutDialog({ open, onOpenChange }: SignOutDialogProps) {
  const navigate = useNavigate()
  const location = useLocation()
  const { t } = useTranslation()
  const handleSignOut = () => {
    resetToken()
    const currentPath = location.href
    navigate({
      to: '/sign-in',
      search: { redirect: currentPath },
      replace: true,
    })
  }

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t('sign_out.title', 'Sign out')}
      desc={t(
        'sign_out.desc',
        'Are you sure you want to sign out? You will need to sign in again to access your account.'
      )}
      confirmText={t('sign_out.confirm', 'Sign out')}
      destructive
      handleConfirm={handleSignOut}
      className="sm:max-w-sm"
    />
  )
}
