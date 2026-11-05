import { mount } from '@vue/test-utils'
import { describe, expect, it, vi, beforeEach } from 'vitest'
import { defineComponent, effectScope, nextTick, reactive, ref } from 'vue'
import { useFormDraft } from './composables/useFormDraft'
import { useInstantSave } from './composables/useInstantSave'
import { resetSnackbar, useSnackbar } from './composables/useSnackbar'

beforeEach(() => resetSnackbar())

describe('useInstantSave', () => {
  it('sets optimistically, saves, and offers Undo that saves the old value', async () => {
    const value = ref(false)
    const save = vi.fn().mockResolvedValue(undefined)
    const { set } = useInstantSave(value, save, { label: 'Saved it' })
    const p = set(true)
    expect(value.value).toBe(true)
    await p
    expect(save).toHaveBeenCalledWith(true)
    const sb = useSnackbar()
    expect(sb.state.current?.text).toBe('Saved it')
    await sb.state.current!.action!.run()
    expect(value.value).toBe(false)
    expect(save).toHaveBeenLastCalledWith(false)
  })

  it('reverts and shows the error when the save fails', async () => {
    const value = ref('a')
    const { set } = useInstantSave(value, () => Promise.reject(new Error('nope')))
    expect(await set('b')).toBe(false)
    expect(value.value).toBe('a')
    expect(useSnackbar().state.current?.text).toBe('nope')
  })

  it('skips Undo when asked', async () => {
    const value = ref(1)
    await useInstantSave(value, async () => {}, { undo: false }).set(2)
    expect(useSnackbar().state.current?.action).toBeUndefined()
  })
})

describe('snackbar queue', () => {
  it('shows one message at a time, then the next', () => {
    const sb = useSnackbar()
    sb.show({ text: 'one', timeoutMs: 0 })
    sb.show({ text: 'two', timeoutMs: 0 })
    expect(sb.state.current?.text).toBe('one')
    sb.dismiss()
    expect(sb.state.current?.text).toBe('two')
    sb.dismiss()
    expect(sb.state.current).toBeNull()
  })
})

describe('useFormDraft', () => {
  const mk = (src: { name: string }, save = vi.fn().mockResolvedValue(undefined)) =>
    effectScope().run(() => useFormDraft(() => src, save, { confirmLeave: () => true }))!

  it('is dirty after an edit, and discard restores the source', async () => {
    const src = reactive({ name: 'a' })
    const f = mk(src)
    expect(f.dirty.value).toBe(false)
    f.draft.value.name = 'b'
    expect(f.dirty.value).toBe(true)
    f.discard()
    expect(f.draft.value.name).toBe('a')
    expect(f.dirty.value).toBe(false)
  })

  it('saves a copy of the draft and becomes clean', async () => {
    const src = reactive({ name: 'a' })
    const save = vi.fn().mockResolvedValue(undefined)
    const f = mk(src, save)
    f.draft.value.name = 'b'
    expect(await f.save()).toBe(true)
    expect(save).toHaveBeenCalledWith({ name: 'b' })
    expect(f.dirty.value).toBe(false)
  })

  it('keeps the draft and reports the error when save fails', async () => {
    const f = mk(reactive({ name: 'a' }), vi.fn().mockRejectedValue(new Error('down')))
    f.draft.value.name = 'b'
    expect(await f.save()).toBe(false)
    expect(f.dirty.value).toBe(true)
    expect(f.error.value).toBe('down')
  })

  it('follows the source while clean, and ignores it while dirty', async () => {
    const src = reactive({ name: 'a' })
    const f = mk(src)
    src.name = 'server'
    await nextTick()
    expect(f.draft.value.name).toBe('server')
    f.draft.value.name = 'mine'
    src.name = 'server2'
    await nextTick()
    expect(f.draft.value.name).toBe('mine')
  })

  it('installs a beforeunload handler only while dirty', async () => {
    const add = vi.spyOn(window, 'addEventListener')
    const remove = vi.spyOn(window, 'removeEventListener')
    const f = mk(reactive({ name: 'a' }))
    f.draft.value.name = 'b'
    await nextTick()
    expect(add.mock.calls.some((c) => c[0] === 'beforeunload')).toBe(true)
    f.discard()
    await nextTick()
    expect(remove.mock.calls.some((c) => c[0] === 'beforeunload')).toBe(true)
    add.mockRestore()
    remove.mockRestore()
  })

  it('works inside a component', () => {
    const C = defineComponent({
      setup() {
        const f = useFormDraft(
          () => ({ n: 1 }),
          async () => {},
          { confirmLeave: () => true },
        )
        return { f }
      },
      template: '<i>{{ f.dirty }}</i>',
    })
    expect(mount(C).text()).toBe('false')
  })
})
