import type React from 'react'
import {
  useCallback,
  useRef,
  useState,
  type ChangeEvent,
  type DragEvent,
  type InputHTMLAttributes,
} from 'react'

export type FileMetadata = {
  name: string
  size: number
  type: string
  url: string
  id: string
}

export type FileWithPreview = {
  file: File | FileMetadata
  id: string
  preview?: string
}

export type FileUploadOptions = {
  maxFiles?: number
  maxSize?: number
  accept?: string
  multiple?: boolean
  initialFiles?: FileMetadata[]
  onFilesChange?: (files: FileWithPreview[]) => void
  onFilesAdded?: (addedFiles: FileWithPreview[]) => void
  onError?: (errors: string[]) => void
}

export type FileUploadState = {
  files: FileWithPreview[]
  isDragging: boolean
  errors: string[]
}

export type FileUploadActions = {
  addFiles: (files: FileList | File[]) => void
  removeFile: (id: string) => void
  clearFiles: () => void
  clearErrors: () => void
  handleDragEnter: (e: DragEvent<HTMLElement>) => void
  handleDragLeave: (e: DragEvent<HTMLElement>) => void
  handleDragOver: (e: DragEvent<HTMLElement>) => void
  handleDrop: (e: DragEvent<HTMLElement>) => void
  handleFileChange: (e: ChangeEvent<HTMLInputElement>) => void
  openFileDialog: () => void
  getInputProps: (
    props?: InputHTMLAttributes<HTMLInputElement>,
  ) => InputHTMLAttributes<HTMLInputElement> & {
    ref: React.Ref<HTMLInputElement>
  }
}

export const useFileUpload = (
  options: FileUploadOptions = {},
): [FileUploadState, FileUploadActions] => {
  const {
    maxFiles = Number.POSITIVE_INFINITY,
    maxSize = Number.POSITIVE_INFINITY,
    accept = '*',
    multiple = false,
    initialFiles = [],
    onFilesChange,
    onFilesAdded,
    onError,
  } = options

  const [state, setState] = useState<FileUploadState>({
    files: initialFiles.map((file) => ({
      file,
      id: file.id,
      preview: file.url,
    })),
    isDragging: false,
    errors: [],
  })

  const inputRef = useRef<HTMLInputElement>(null)

  const validateFile = useCallback(
    (file: File | FileMetadata): string | null => {
      const size = file instanceof File ? file.size : file.size

      if (size > maxSize) {
        return `File "${file.name}" exceeds the maximum size of ${formatBytes(
          maxSize,
        )}.`
      }

      if (accept !== '*') {
        const acceptedTypes = accept.split(',').map((t) => t.trim())
        const fileType = file instanceof File ? file.type || '' : file.type
        const ext = `.${file.name.split('.').pop()}`

        const ok = acceptedTypes.some((type) => {
          if (type.startsWith('.')) return ext.toLowerCase() === type.toLowerCase()
          if (type.endsWith('/*')) return fileType.startsWith(type.replace('/*', '/'))
          return fileType === type
        })

        if (!ok) {
          return `File "${file.name}" is not an accepted file type.`
        }
      }

      return null
    },
    [accept, maxSize],
  )

  const createPreview = useCallback((file: File | FileMetadata) => {
    if (file instanceof File) {
      return URL.createObjectURL(file)
    }
    return file.url
  }, [])

  const generateId = useCallback((file: File | FileMetadata) => {
    if (file instanceof File) {
      return `${file.name}-${Date.now()}-${Math.random()
        .toString(36)
        .slice(2, 9)}`
    }
    return file.id
  }, [])

  const clearFiles = useCallback(() => {
    setState((prev) => {
      prev.files.forEach((f) => {
        if (
          f.preview &&
          f.file instanceof File &&
          f.file.type.startsWith('image/')
        ) {
          URL.revokeObjectURL(f.preview)
        }
      })

      if (inputRef.current) {
        inputRef.current.value = ''
      }

      onFilesChange?.([])
      return { ...prev, files: [], errors: [] }
    })
  }, [onFilesChange])

  const addFiles = useCallback(
    (files: FileList | File[]) => {
      if (!files || files.length === 0) return
      if (inputRef.current?.disabled) return

      const incoming = Array.from(files)
      const errors: string[] = []
      const valid: FileWithPreview[] = []

      if (!multiple) {
        clearFiles()
      }

      if (
        multiple &&
        maxFiles !== Number.POSITIVE_INFINITY &&
        state.files.length + incoming.length > maxFiles
      ) {
        errors.push(`You can only upload up to ${maxFiles} files.`)
        onError?.(errors)
        setState((p) => ({ ...p, errors }))
        return
      }

      for (const file of incoming) {
        const err = validateFile(file)
        if (err) {
          errors.push(err)
          continue
        }

        valid.push({
          file,
          id: generateId(file),
          preview: createPreview(file),
        })
      }

      if (valid.length > 0) {
        onFilesAdded?.(valid)

        setState((prev) => {
          const next = multiple ? [...prev.files, ...valid] : valid
          onFilesChange?.(next)
          return { ...prev, files: next, errors }
        })
      } else if (errors.length > 0) {
        onError?.(errors)
        setState((p) => ({ ...p, errors }))
      }

      if (inputRef.current) {
        inputRef.current.value = ''
      }
    },
    [
      state.files,
      multiple,
      maxFiles,
      validateFile,
      generateId,
      createPreview,
      clearFiles,
      onFilesChange,
      onFilesAdded,
      onError,
    ],
  )

  const removeFile = useCallback(
    (id: string) => {
      setState((prev) => {
        const file = prev.files.find((f) => f.id === id)
        if (
          file?.preview &&
          file.file instanceof File &&
          file.file.type.startsWith('image/')
        ) {
          URL.revokeObjectURL(file.preview)
        }

        const next = prev.files.filter((f) => f.id !== id)
        onFilesChange?.(next)
        return { ...prev, files: next, errors: [] }
      })
    },
    [onFilesChange],
  )

  const clearErrors = useCallback(() => {
    setState((p) => ({ ...p, errors: [] }))
  }, [])

  const handleDragEnter = useCallback((e: DragEvent<HTMLElement>) => {
    e.preventDefault()
    e.stopPropagation()
    if (inputRef.current?.disabled) return
    setState((p) => ({ ...p, isDragging: true }))
  }, [])

  const handleDragLeave = useCallback((e: DragEvent<HTMLElement>) => {
    e.preventDefault()
    e.stopPropagation()
    if (e.currentTarget.contains(e.relatedTarget as Node)) return
    setState((p) => ({ ...p, isDragging: false }))
  }, [])

  const handleDragOver = useCallback((e: DragEvent<HTMLElement>) => {
    e.preventDefault()
    e.stopPropagation()
  }, [])

  const handleDrop = useCallback(
    (e: DragEvent<HTMLElement>) => {
      e.preventDefault()
      e.stopPropagation()
      setState((p) => ({ ...p, isDragging: false }))

      if (inputRef.current?.disabled) return

      if (e.dataTransfer.files?.length) {
        addFiles(
          multiple ? e.dataTransfer.files : [e.dataTransfer.files[0]],
        )
      }
    },
    [addFiles, multiple],
  )

  const handleFileChange = useCallback(
    (e: ChangeEvent<HTMLInputElement>) => {
      if (e.target.disabled) return
      if (e.target.files?.length) {
        addFiles(e.target.files)
      }
    },
    [addFiles],
  )

  const openFileDialog = useCallback(() => {
    if (!inputRef.current || inputRef.current.disabled) return
    inputRef.current.click()
  }, [])

  const getInputProps = useCallback(
    (props: InputHTMLAttributes<HTMLInputElement> = {}) => ({
      ...props,
      type: 'file' as const,
      accept: props.accept ?? accept,
      multiple: props.multiple ?? multiple,
      disabled: props.disabled,
      onChange: handleFileChange,
      ref: inputRef,
    }),
    [accept, multiple, handleFileChange],
  )

  return [
    state,
    {
      addFiles,
      removeFile,
      clearFiles,
      clearErrors,
      handleDragEnter,
      handleDragLeave,
      handleDragOver,
      handleDrop,
      handleFileChange,
      openFileDialog,
      getInputProps,
    },
  ]
}

export const formatBytes = (bytes: number, decimals = 2): string => {
  if (bytes === 0) return '0 Bytes'
  const k = 1024
  const dm = decimals < 0 ? 0 : decimals
  const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return `${parseFloat((bytes / k ** i).toFixed(dm))}${sizes[i]}`
}
