import { ref, type Ref } from 'vue'
import { useSnackbar } from './useSnackbar'

export interface InstantSaveOptions {
  /** Offer Undo after a successful save (default true). */
  undo?: boolean
  /** Snackbar text on success (default "Saved"). */
  label?: string
}

/**
 * Single controls apply at once (D-096): set the value optimistically, await the save, then show
 * a snackbar with Undo. Undo saves the old value; a failed save reverts and shows the error.
 * Bind with `:model-value="target"` and `@update:model-value="set"`.
 */
export function useInstantSave<T>(
  target: Ref<T>,
  save: (value: T) => Promise<void>,
  opts: InstantSaveOptions = {},
) {
  const snackbar = useSnackbar()
  const saving = ref(false)

  async function set(value: T): Promise<boolean> {
    const old = target.value
    target.value = value
    saving.value = true
    try {
      await save(value)
    } catch (e) {
      target.value = old
      snackbar.show({ text: e instanceof Error ? e.message : 'Could not save' })
      return false
    } finally {
      saving.value = false
    }
    snackbar.show({
      text: opts.label ?? 'Saved',
      ...(opts.undo === false
        ? {}
        : {
            action: {
              label: 'Undo',
              run: async () => {
                target.value = old
                try {
                  await save(old)
                } catch (e) {
                  target.value = value
                  snackbar.show({ text: e instanceof Error ? e.message : 'Could not undo' })
                }
              },
            },
          }),
    })
    return true
  }
  return { set, saving }
}
