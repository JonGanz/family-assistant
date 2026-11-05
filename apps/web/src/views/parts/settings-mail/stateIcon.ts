import type { IconName } from '../../../ui'

/** The icon of a source or subsystem state chip: the state is never colour alone. */
export function stateIcon(state: string): IconName {
  if (state === 'ok') return 'success'
  if (state === 'error' || state === 'failing') return 'error'
  if (state === 'needs_reauth' || state === 'attention') return 'warning'
  return 'info'
}
