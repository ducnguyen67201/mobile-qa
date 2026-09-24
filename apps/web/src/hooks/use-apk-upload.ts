/** Owns transient transfer state; every retry reconciles the saved upload with the server. */
import { useEffect, useRef, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { useSearchParams } from 'react-router'
import type { MultipartUpload, UploadResponse } from '@/api/generated/types.gen'
import { completeUpload, createUpload, getUpload, transferUpload } from '@/api/setup'
import { abortMultipartUpload, getMultipartUpload } from '@/api/multipart'
import { ApiClientError } from '@/api/runtime'
import { uploadMultipartFile, type TransferProgress } from '@/lib/multipart-upload'
import { formatBytes } from '@/lib/format'

async function savedMultipart(appId: string, uploadId: string, signal: AbortSignal) {
  try {
    const descriptor = await getMultipartUpload(appId, uploadId, signal)
    if (descriptor.upload_id !== uploadId)
      throw new Error('The saved upload reference does not match.')
    return descriptor
  } catch (reason) {
    // Only an absent descriptor proves this is a legacy transfer. Authentication,
    // network and validation failures must retain the saved upload reference.
    if (reason instanceof ApiClientError && reason.status === 404) return null
    throw reason
  }
}

export function useApkUpload(appId: string, maxBytes: number, multipartEnabled = false) {
  const [params, setParams] = useSearchParams()
  const uploadId = params.get('upload')
  const client = useQueryClient()
  const [file, setFile] = useState<File | null>(null)
  const [upload, setUpload] = useState<UploadResponse | null>(null)
  const [multipart, setMultipart] = useState<MultipartUpload | null>(null)
  const [phase, setPhase] = useState('')
  const [error, setError] = useState<unknown>(null)
  const [inputError, setInputError] = useState('')
  const [progress, setProgress] = useState<TransferProgress | null>(null)
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
    const controller = new AbortController()
    if (uploadId)
      void getUpload(appId, uploadId, controller.signal)
        .then(async (value) => {
          if (!current) return
          setUpload(value)
          setMultipart(null)
          const descriptor =
            value.state === 'receiving'
              ? await savedMultipart(appId, uploadId, controller.signal)
              : null
          if (!current) return
          setMultipart(descriptor)
          setError(null)
        })
        .catch((reason) => {
          if (current) setError(reason)
        })
    return () => {
      current = false
      controller.abort()
    }
  }, [appId, uploadId])
  const updateUrl = (id: string, buildId?: string) => {
    const next = new URLSearchParams(paramsRef.current)
    next.set('upload', id)
    if (buildId) next.set('build', buildId)
    setParams(next, { replace: true })
  }
  const inspect = async () => {
    if (!uploadId || abort.current) return
    abort.current = new AbortController()
    setPhase('Checking saved upload…')
    setError(null)
    try {
      const value = await getUpload(appId, uploadId, abort.current.signal)
      if (!active.current) return
      setUpload(value)
      setMultipart(null)
      const descriptor =
        value.state === 'receiving'
          ? await savedMultipart(appId, uploadId, abort.current.signal)
          : null
      if (active.current) setMultipart(descriptor)
    } catch (reason) {
      if (active.current) setError(reason)
    } finally {
      abort.current = null
      if (active.current) setPhase('')
    }
  }
  const run = async () => {
    if (abort.current) return
    setError(null)
    setInputError('')
    setProgress(null)
    abort.current = new AbortController()
    const signal = abort.current.signal
    try {
      setPhase('Checking upload…')
      // Always reconcile an existing upload before retrying a mutation after uncertainty.
      let current = uploadId ? await getUpload(appId, uploadId, signal) : null
      if (!active.current) return
      setUpload(current)
      setMultipart(null)
      if (current?.state === 'expired') {
        setUpload(current)
        setInputError('This upload expired. Start a new upload.')
        return
      }
      // Resolve saved sessions independently of cached settings so retries use
      // their original transfer protocol.
      const existingMultipart =
        current?.state === 'receiving' ? await savedMultipart(appId, current.id, signal) : null
      if (!active.current) return
      setMultipart(existingMultipart)
      if (current?.state === 'receiving' && !existingMultipart) {
        setInputError('The server is still receiving this file. Check its status before retrying.')
        return
      }
      if (!current || current.state === 'pending' || current.state === 'receiving') {
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
        const useMultipart = !!existingMultipart || multipartEnabled
        setPhase(useMultipart ? 'Checking saved parts and transferring APK…' : 'Transferring APK…')
        current = useMultipart
          ? await uploadMultipartFile(
              appId,
              current.id,
              file,
              signal,
              (value) => {
                if (active.current) {
                  setProgress(value)
                  if (value.completedBytes === value.totalBytes)
                    setPhase('Verifying stored APK bytes…')
                }
              },
              existingMultipart ?? undefined,
            )
          : await transferUpload(appId, current.id, file, signal)
        if (!active.current) return
        setUpload(current)
      }
      setPhase('Validating stored APK…')
      const build = await completeUpload(appId, current.id, signal)
      if (!active.current) return
      updateUrl(current.id, build.id)
      client.setQueryData(['build', appId, build.id], build)
      void client.invalidateQueries({ queryKey: ['builds', appId] })
      const recovered = await getUpload(appId, current.id, signal)
      if (active.current) {
        setUpload(recovered)
        setFile(null)
      }
    } catch (reason) {
      if (active.current) {
        if (signal.aborted)
          setInputError(
            'Request stopped. Your upload reference is saved; check its status or resume with the original file.',
          )
        else setError(reason)
      }
    } finally {
      abort.current = null
      if (active.current) setPhase('')
    }
  }
  const clear = async () => {
    if (abort.current) return
    if (uploadId && (!upload || ['pending', 'receiving'].includes(upload.state))) {
      abort.current = new AbortController()
      setPhase('Discarding incomplete upload…')
      try {
        const current = await getUpload(appId, uploadId, abort.current.signal)
        if (!active.current) return
        setUpload(current)
        setMultipart(null)
        if (['pending', 'receiving'].includes(current.state)) {
          const descriptor = await savedMultipart(appId, uploadId, abort.current.signal)
          if (!active.current) return
          setMultipart(descriptor)
          if (descriptor) await abortMultipartUpload(appId, uploadId, abort.current.signal)
        }
      } catch (reason) {
        // A saved upload can exist before its first multipart session is created.
        // There are no provider parts to discard when that session is absent.
        if (!(reason instanceof ApiClientError && reason.status === 404)) {
          if (active.current) setError(reason)
          return
        }
      } finally {
        abort.current = null
        if (active.current) setPhase('')
      }
    }
    if (!active.current) return
    const next = new URLSearchParams(paramsRef.current)
    next.delete('upload')
    setParams(next, { replace: true })
    setUpload(null)
    setMultipart(null)
    setFile(null)
    setError(null)
    setInputError('')
    setProgress(null)
  }
  const selectFile = (value: File | null) => {
    setFile(value)
    setInputError('')
  }
  const stop = () => abort.current?.abort()
  return {
    uploadId,
    upload,
    canResumeMultipart: multipart?.state === 'uploading',
    phase,
    error,
    inputError,
    progress,
    selectFile,
    run,
    inspect,
    clear,
    stop,
    updateUrl,
  }
}
