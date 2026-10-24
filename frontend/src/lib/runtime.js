export const storageMode = import.meta.env.VITE_STORAGE_MODE === 'local' ? 'local' : 'api'
export const isLocalMode = storageMode === 'local'

