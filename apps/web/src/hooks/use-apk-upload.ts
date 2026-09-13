/** Owns transient transfer state; every retry reconciles the saved upload with the server. */
import { useEffect, useRef, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useSearchParams } from 'react-router'
import type { UploadResponse } from '@/api/generated/types.gen'
import { completeUpload, createUpload, getUpload, transferUpload } from '@/api/setup'
import { formatBytes } from '@/lib/format'

export function useApkUpload(appId: string, maxBytes: number) {
  const [params, setParams] = useSearchParams()
  const uploadId = params.get('upload')
  const client = useQueryClient()
  const [file, setFile] = useState<File | null>(null)
  const [upload, setUpload] = useState<UploadResponse | null>(null)
  const [phase, setPhase] = useState('')
  const [error, setError] = useState<unknown>(null)
  const [inputError, setInputError] = useState('')
  const abort = useRef<AbortController | null>(null)
  const active = useRef(true)
  const paramsRef = useRef(params)
  paramsRef.current = params
  useEffect(() => {
    active.current = true
    return () => {
      active.current = false
      abort.current?.abort()
    }
  }, [])
  useEffect(() => {
    let current = true
    if (uploadId)
      void getUpload(appId, uploadId)
        .then((value) => {
          if (current) {
            setUpload(value)
            setError(null)
          }
        })
        .catch((reason) => {
          if (current) setError(reason)
        })
    return () => {
      current = false
    }
  }, [appId, uploadId])
  const updateUrl = (id: string, buildId?: string) => {
    const next = new URLSearchParams(paramsRef.current)
    next.set('upload', id)
    if (buildId) next.set('build', buildId)
    setParams(next, { replace: true })
  }
  const inspect = async () => {
    if (!uploadId) return
    setPhase('Checking saved upload…')
    setError(null)
    try {
      const value = await getUpload(appId, uploadId)
      if (active.current) setUpload(value)
    } catch (reason) {
      if (active.current) setError(reason)
    } finally {
      if (active.current) setPhase('')
    }
  }
  const run = async () => {
    setError(null)
    setInputError('')
    abort.current = new AbortController()
    const signal = abort.current.signal
    try {
      setPhase('Checking upload…')
      // Always reconcile an existing upload before retrying a mutation after uncertainty.
      let current = uploadId ? await getUpload(appId, uploadId) : null
      if (!active.current) return
      if (current?.state === 'expired') {
        setUpload(current)
        setInputError('This upload expired. Start a new upload.')
        return
      }
      if (current?.state === 'receiving') {
        setUpload(current)
        setInputError('The server is still receiving this file. Check its status before retrying.')
        return
      }
      if (!current || current.state === 'pending') {
        if (!file) {
          setInputError('Choose the APK file to continue.')
          return
        }
        if (file.size === 0 || file.size > maxBytes) {
          setInputError(`Choose a nonempty APK up to ${formatBytes(maxBytes)}.`)
          return
        }
        if (
          current &&
          (file.name !== current.original_filename || file.size !== current.expected_size)
        ) {
          setInputError(
            'Reselect the original file with the same name and size, or start a new upload.',
          )
          return
        }
        if (!current) {
          setPhase('Creating upload…')
          current = await createUpload(appId, file, signal)
          if (!active.current) return
          updateUrl(current.id)
          setUpload(current)
        }
        setPhase('Transferring APK…')
        current = await transferUpload(appId, current.id, file, signal)
        if (!active.current) return
        setUpload(current)
      }
      setPhase('Validating stored APK…')
      const build = await completeUpload(appId, current.id, signal)
      if (!active.current) return
      updateUrl(current.id, build.id)
      client.setQueryData(['build', appId, build.id], build)
      void client.invalidateQueries({ queryKey: ['builds', appId] })
      const recovered = await getUpload(appId, current.id)
      if (active.current) {
        setUpload(recovered)
        setFile(null)
      }
    } catch (reason) {
      if (active.current) setError(reason)
    } finally {
      if (active.current) setPhase('')
    }
  }
  const clear = () => {
    const next = new URLSearchParams(paramsRef.current)
    next.delete('upload')
    setParams(next, { replace: true })
    setUpload(null)
    setFile(null)
    setError(null)
    setInputError('')
  }
  const selectFile = (value: File | null) => {
    setFile(value)
    setInputError('')
  }
  const stop = () => abort.current?.abort()
  return {
    uploadId,
    upload,
    phase,
    error,
    inputError,
    selectFile,
    run,
    inspect,
    clear,
    stop,
    updateUrl,
  }
}
