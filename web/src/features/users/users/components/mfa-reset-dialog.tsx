import { useTranslation } from 'react-i18next'
import { useMutation, useQueryClient } from '@tanstack/react-query'
import { AxiosError } from 'axios'
import { toast } from '@/hooks/use-toast'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { mfaAdminReset, User } from '@/api/users/api'

interface Props {
  open: boolean
  onOpenChange: (open: boolean) => void
  currentRow: User
}

export function UserMfaResetDialog({ open, onOpenChange, currentRow }: Props) {
  const { t } = useTranslation()
  const queryClient = useQueryClient()

  const resetMutation = useMutation({
    mutationFn: () => mfaAdminReset(currentRow.id),
    onSuccess: () => {
      toast({
        title: t('users.actions.reset_mfa.success_title'),
        description: t('users.actions.reset_mfa.success_desc', { name: currentRow.username }),
      })
      queryClient.invalidateQueries({ queryKey: ['user-list'] })
      onOpenChange(false)
    },
    onError: (error: AxiosError) => {
      const errorMessage =
        (error.response?.data as { message?: string })?.message ||
        error.message ||
        t('users.actions.reset_mfa.failed')

      toast({
        variant: 'destructive',
        title: t('users.actions.reset_mfa.failed'),
        description: errorMessage,
      })
    },
  })

  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      handleConfirm={() => resetMutation.mutate()}
      destructive
      isLoading={resetMutation.isPending}
      title={t('users.actions.reset_mfa.title')}
      desc={t('users.actions.reset_mfa.confirm_msg', {
        name: currentRow.username,
        id: currentRow.id,
      })}
      confirmText={t('users.actions.reset_mfa.button_confirm')}
    />
  )
}
