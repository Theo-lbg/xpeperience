import { ref } from 'vue'
import { ApiError } from './api'

/** Gestion commune des formulaires : erreurs par champ, message général, état occupé. */
export function useForm() {
  const errors = ref<Record<string, string>>({})
  const message = ref('')
  const busy = ref(false)
  const conflict = ref<ApiError | null>(null)

  async function submit<T>(fn: () => Promise<T>, known: string[] = []): Promise<T | undefined> {
    busy.value = true
    errors.value = {}
    message.value = ''
    conflict.value = null
    try {
      return await fn()
    } catch (e) {
      if (e instanceof ApiError) {
        if (e.status === 409) conflict.value = e
        errors.value = e.fieldMap
        const unknown = e.fields.filter((f) => known.length && !known.includes(f.field))
        if (e.status !== 409 && (!e.fields.length || unknown.length)) message.value = e.message
        else if (e.status === 409 && !e.details?.duplicates) message.value = e.message
      } else message.value = 'Enregistrement impossible. Réessayez.'
    } finally {
      busy.value = false
    }
  }

  return { errors, message, busy, conflict, submit }
}
