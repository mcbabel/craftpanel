import { type Kyros, ModrinthApiError, type UploadState } from '@modrinth/api-client'
import {
	defineMessages,
	type MessageDescriptor,
	useVIntl,
} from '@modrinth/ui/src/composables/i18n.ts'
import { hasServerPermission } from '@modrinth/ui/src/composables/server-permissions.ts'
import { useReadyState } from '@modrinth/ui/src/composables/use-ready-state.ts'
import type { FileManagerContext } from '@modrinth/ui/src/layouts/shared/files-tab/providers/file-manager.ts'
import { provideFileManager } from '@modrinth/ui/src/layouts/shared/files-tab/providers/index.ts'
import type {
	EditingFile,
	ExtractDryRunResult,
	FileItem,
	FileOperation,
} from '@modrinth/ui/src/layouts/shared/files-tab/types.ts'
import type { BusyReason } from '@modrinth/ui/src/providers/server-context.ts'
import { injectNotificationManager } from '@modrinth/ui/src/providers/web-notifications.ts'
import { commonMessages } from '@modrinth/ui/src/utils/common-messages.ts'
import { queryOptions, useQuery, useQueryClient } from '@tanstack/vue-query'
import type { ComputedRef, MaybeRefOrGetter, Ref } from 'vue'
import { computed, onScopeDispose, ref, toValue, watch } from 'vue'

import { api, hasErrorCode, isApiRequestError } from '@/api'
import type {
	ApiFileItem,
	BusyReasonCode,
	FilesMetaResponse,
	Operation,
	OperationState,
	PermissionMask,
	ServerEventSource,
	Ulid,
	UploadHandle,
	ZipRequest,
} from '@/api'

import { FILE_OPERATION_KINDS, toFileOperation } from './server-context'

const FILES_KEY = 'server-files'
const DIRECTORY_STALE_MS = 30_000
const TEXT_STALE_MS = 2_000

const TERMINAL_STATES = new Set<OperationState>(['done', 'failed', 'cancelled'])

export const BUSY_MESSAGES: Record<BusyReasonCode, MessageDescriptor> = defineMessages({
	installing: { id: 'servers.busy.installing', defaultMessage: 'Server is installing' },
	syncing_content: {
		id: 'servers.busy.syncing-content',
		defaultMessage: 'Content sync in progress',
	},
	backup_creating: {
		id: 'servers.busy.backup-creating',
		defaultMessage: 'Backup creation in progress',
	},
	backup_restoring: {
		id: 'servers.busy.backup-restoring',
		defaultMessage: 'Backup restore in progress',
	},
	deleting: { id: 'servers.busy.deleting', defaultMessage: 'Server is being deleted' },
})

const messages = defineMessages({
	backgroundTask: { id: 'servers.busy.unknown', defaultMessage: 'Background task running' },
	uploadConflict: {
		id: 'craftpanel.files.upload-conflict',
		defaultMessage: '{file} already exists in this folder.',
	},
	uploadTooLarge: {
		id: 'craftpanel.files.upload-too-large',
		defaultMessage: '{file} is larger than the upload limit.',
	},
	cancelFailed: {
		id: 'craftpanel.files.cancel-failed',
		defaultMessage: 'Could not cancel the extraction',
	},
	zipFailed: { id: 'craftpanel.files.zip-failed', defaultMessage: 'Could not create the ZIP' },
	zipReady: { id: 'craftpanel.files.zip-ready', defaultMessage: 'ZIP ready' },
	zipReadyText: {
		id: 'craftpanel.files.zip-ready-text',
		defaultMessage: '{file} has been created.',
	},
	zipTooLarge: {
		id: 'craftpanel.files.zip-too-large',
		defaultMessage: 'This is more than the panel packs into one ZIP (20 GiB or 200,000 files).',
	},
	zipNoRoom: {
		id: 'craftpanel.files.zip-no-room',
		defaultMessage: 'There is not enough room left in your disk budget for this ZIP.',
	},
})

interface DirectoryListing {
	items: ApiFileItem[]
	partial: boolean
}

export type PanelClient = Pick<typeof api, 'files' | 'operations'>

export interface FileManagerOptions {
	serverId: Ulid
	socket: ServerEventSource
	permissions: MaybeRefOrGetter<PermissionMask>
	uploadState?: Ref<UploadState>
	client?: PanelClient
}

export interface ServerFileManager {
	context: FileManagerContext
	basePath: ComputedRef<string>
	pending: Readonly<Ref<boolean>>
	refresh: () => Promise<void>
	uploadState: Ref<UploadState>
	cancelUpload: () => void
	activeOperations: ComputedRef<FileOperation[]>
	dismissOperation: (id: string, action: 'dismiss' | 'cancel') => Promise<void>
	busyCodes: Ref<BusyReasonCode[]>
	busyReasons: ComputedRef<BusyReason[]>
	canWriteFiles: ComputedRef<boolean>
}

function normalizePath(path: string): string {
	return `/${path.split('/').filter(Boolean).join('/')}`
}

function joinPath(directory: string, name: string): string {
	return normalizePath(`${directory}/${name}`)
}

function parentOf(path: string): string {
	return normalizePath(path.split('/').slice(0, -1).join('/'))
}

function nameOf(path: string): string {
	return normalizePath(path).split('/').pop() ?? ''
}

function statType(type: FileItem['type']): Kyros.Files.v1.DescendantType {
	if (type === 'directory') return 'directory'
	if (type === 'symlink') return 'symlink'
	return 'regular'
}

function idleUpload(): UploadState {
	return {
		isUploading: false,
		currentFileName: null,
		currentFileProgress: 0,
		uploadedBytes: 0,
		totalBytes: 0,
		completedFiles: 0,
		totalFiles: 0,
	}
}

export function startDownload(url: string, fileName: string): void {
	const link = document.createElement('a')
	link.href = url
	link.download = fileName
	document.body.append(link)
	link.click()
	link.remove()
}

function isAborted(error: unknown): boolean {
	return error instanceof Error && error.name === 'AbortError'
}

export function provideServerFileManager(options: FileManagerOptions): ServerFileManager {
	const { serverId, socket } = options
	const { files, operations: operationsApi } = options.client ?? api
	const { formatMessage } = useVIntl()
	const { addNotification } = injectNotificationManager()
	const queryClient = useQueryClient()

	const currentPath = ref('/')
	const editingFile = ref<EditingFile | null>(null)
	const operations = ref<Operation[]>([])
	const busyCodes = ref<BusyReasonCode[]>([])
	const dismissed = ref(new Set<string>())
	const uploadState = options.uploadState ?? ref<UploadState>(idleUpload())

	let running: UploadHandle<void> | null = null
	let uploadCancelled = false
	const zipsStartedHere = new Set<Ulid>()

	function notifyFailure(title: MessageDescriptor, error: unknown): void {
		addNotification({
			title: formatMessage(title),
			text: error instanceof Error ? error.message : undefined,
			type: 'error',
			errorCode: isApiRequestError(error) ? error.code : undefined,
		})
	}

	const metaOptions = queryOptions({
		queryKey: [FILES_KEY, serverId, 'meta'],
		queryFn: () => files.meta(serverId),
		staleTime: Infinity,
		retry: false,
	})
	const meta = useQuery(metaOptions)

	async function limits(): Promise<FilesMetaResponse | null> {
		try {
			return await queryClient.ensureQueryData(metaOptions)
		} catch {
			return null
		}
	}

	const directoryKey = (path: string) => [FILES_KEY, serverId, 'directory', path]

	function directoryOptions(path: string) {
		return queryOptions({
			queryKey: directoryKey(path),
			queryFn: async ({ signal }): Promise<DirectoryListing> => {
				const page = await files.listAll(serverId, path, { signal })
				return { items: page.items, partial: false }
			},
			staleTime: (query) => (query.state.data?.partial ? 0 : DIRECTORY_STALE_MS),
			retry: false,
		})
	}

	function textOptions(path: string) {
		return queryOptions({
			queryKey: [FILES_KEY, serverId, 'text', normalizePath(path)],
			queryFn: async ({ signal }) =>
				files.readText(
					serverId,
					{ path: normalizePath(path), max_bytes: (await limits())?.max_text_bytes },
					{ signal },
				),
			staleTime: TEXT_STALE_MS,
			retry: false,
		})
	}

	const directory = useQuery(computed(() => directoryOptions(currentPath.value)))

	const items = computed<FileItem[]>(() => directory.data.value?.items ?? [])
	const loading = computed(() => directory.isLoading.value)
	const error = computed(() => directory.error.value)
	const basePath = computed(() => meta.data.value?.root_path ?? '')

	function refresh(): Promise<void> {
		return queryClient.invalidateQueries({ queryKey: [FILES_KEY, serverId, 'directory'] })
	}

	function navigateTo(path: string): void {
		currentPath.value = normalizePath(path)
	}

	function startEditing(file: EditingFile): void {
		editingFile.value = { name: file.name, path: normalizePath(file.path).slice(1) }
	}

	function stopEditing(): void {
		editingFile.value = null
	}

	async function createItem(name: string, type: 'file' | 'directory'): Promise<void> {
		const target = currentPath.value
		try {
			const { item } = await files.create(serverId, { path: joinPath(target, name), type })
			const key = directoryKey(target)
			if (queryClient.getQueryData<DirectoryListing>(key)) {
				queryClient.setQueryData<DirectoryListing>(key, (listing) =>
					listing ? { ...listing, items: [...listing.items, item] } : listing,
				)
			} else {
				await refresh()
			}
		} catch (failure) {
			notifyFailure(commonMessages.createFailedLabel, failure)
		}
	}

	async function renameItem(path: string, newName: string): Promise<void> {
		try {
			await files.move(serverId, {
				source: normalizePath(path),
				destination: joinPath(parentOf(path), newName),
			})
			await refresh()
		} catch (failure) {
			notifyFailure(commonMessages.renameFailedLabel, failure)
		}
	}

	async function moveItem(source: string, destination: string): Promise<void> {
		try {
			await files.move(serverId, {
				source: normalizePath(source),
				destination: normalizePath(destination),
			})
			await refresh()
		} catch (failure) {
			notifyFailure(commonMessages.moveFailedLabel, failure)
		}
	}

	async function deleteItem(path: string, recursive: boolean): Promise<void> {
		try {
			await files.remove(serverId, { path: normalizePath(path), recursive })
			await refresh()
		} catch (failure) {
			notifyFailure(commonMessages.deleteFailedLabel, failure)
		}
	}

	function readFile(path: string): Promise<string> {
		return queryClient.fetchQuery(textOptions(path))
	}

	async function readFileAsBlob(path: string): Promise<Blob> {
		return files.readBlob(serverId, {
			path: normalizePath(path),
			max_bytes: (await limits())?.max_text_bytes,
		})
	}

	async function writeFile(path: string, content: string): Promise<void> {
		await files.write(serverId, { path: normalizePath(path), on_conflict: 'overwrite' }, content)
		queryClient.setQueryData(textOptions(path).queryKey, content)
		await refresh()
	}

	async function statFile(path: string): Promise<Kyros.Files.v1.FileStatResponse> {
		const normalized = normalizePath(path)
		const listing = await queryClient.fetchQuery(directoryOptions(parentOf(normalized)))
		const item = listing.items.find((entry) => entry.name === nameOf(normalized))
		if (!item) throw new ModrinthApiError('no such file', { statusCode: 404 })
		return {
			name: item.name,
			full_path: item.path,
			size_bytes: item.size ?? 0,
			type: statType(item.type),
			mtime: new Date(item.modified * 1000).toISOString(),
			ctime: new Date(item.created * 1000).toISOString(),
		}
	}

	async function startZip(request: ZipRequest): Promise<void> {
		try {
			const { operation } = await files.zip(serverId, request)
			zipsStartedHere.add(operation.id)
			const known = operations.value.find((candidate) => candidate.id === operation.id)
			if (known && TERMINAL_STATES.has(known.state)) settleZip(known)
		} catch (failure) {
			addNotification({
				title: formatMessage(messages.zipFailed),
				text: hasErrorCode(failure, 'archive_too_large')
					? formatMessage(messages.zipTooLarge)
					: hasErrorCode(failure, 'disk_limit_reached')
						? formatMessage(messages.zipNoRoom)
						: failure instanceof Error
							? failure.message
							: undefined,
				type: 'error',
				errorCode: isApiRequestError(failure) ? failure.code : undefined,
			})
		}
	}

	function zipFolder(path: string): Promise<void> {
		return startZip({ parent: parentOf(path), include: [nameOf(path)], on_conflict: 'rename' })
	}

	function zipPaths(parent: string, include: string[], target: string): Promise<void> {
		return startZip({ parent: normalizePath(parent), include, target, on_conflict: 'fail' })
	}

	async function downloadFile(path: string, fileName: string): Promise<void> {
		startDownload(files.contentUrl(serverId, { path: normalizePath(path), download: 1 }), fileName)
	}

	async function uploadFiles(input: File[]): Promise<void> {
		if (input.length === 0) return

		const target = currentPath.value
		const maxBytes = (await limits())?.max_upload_bytes
		uploadCancelled = false
		uploadState.value = {
			...idleUpload(),
			isUploading: true,
			totalBytes: input.reduce((sum, file) => sum + file.size, 0),
			totalFiles: input.length,
		}

		let sent = 0
		for (const file of input) {
			if (uploadCancelled) break
			uploadState.value = {
				...uploadState.value,
				currentFileName: file.name,
				currentFileProgress: 0,
			}

			if (maxBytes !== undefined && file.size > maxBytes) {
				addNotification({
					title: formatMessage(commonMessages.uploadFailedLabel),
					text: formatMessage(messages.uploadTooLarge, { file: file.name }),
					type: 'error',
				})
				continue
			}

			let handle: UploadHandle<void> | null = null
			try {
				handle = files
					.upload(serverId, { path: joinPath(target, file.name), on_conflict: 'fail' }, file)
					.onProgress(({ loaded, progress }) => {
						uploadState.value = {
							...uploadState.value,
							currentFileProgress: progress,
							uploadedBytes: sent + loaded,
						}
					})
				running = handle
				await handle.promise
				sent += file.size
				uploadState.value = {
					...uploadState.value,
					currentFileProgress: 1,
					uploadedBytes: sent,
					completedFiles: uploadState.value.completedFiles + 1,
				}
			} catch (failure) {
				if (isAborted(failure)) break
				addNotification({
					title: formatMessage(commonMessages.uploadFailedLabel),
					text: hasErrorCode(failure, 'already_exists')
						? formatMessage(messages.uploadConflict, { file: file.name })
						: failure instanceof Error
							? failure.message
							: undefined,
					type: 'error',
					errorCode: isApiRequestError(failure) ? failure.code : undefined,
				})
			} finally {
				if (running === handle) running = null
			}
		}

		uploadState.value = { ...uploadState.value, isUploading: false, currentFileName: null }
		await refresh()
	}

	function cancelUpload(): void {
		uploadCancelled = true
		running?.cancel()
	}

	async function extractFile(
		path: string,
		override: boolean,
		dry: boolean,
	): Promise<ExtractDryRunResult | void> {
		const request = { path: normalizePath(path), override }
		if (dry) return files.extractDryRun(serverId, request)
		await files.extract(serverId, request)
	}

	function prefetchDirectory(path: string): void {
		const normalized = normalizePath(path)
		const key = directoryKey(normalized)
		if (queryClient.getQueryData<DirectoryListing>(key)) return
		void files
			.list(serverId, { path: normalized })
			.then((page) => {
				if (queryClient.getQueryData<DirectoryListing>(key)) return
				queryClient.setQueryData<DirectoryListing>(key, {
					items: page.items,
					partial: page.has_more,
				})
			})
			.catch(() => undefined)
	}

	function prefetchFile(path: string): void {
		void queryClient.prefetchQuery(textOptions(path))
	}

	const activeOperations = computed<FileOperation[]>(() =>
		operations.value
			.filter(
				(operation) =>
					FILE_OPERATION_KINDS.has(operation.kind) &&
					operation.state !== 'cancelled' &&
					!dismissed.value.has(operation.id),
			)
			.map(toFileOperation),
	)

	async function dismissOperation(id: string, action: 'dismiss' | 'cancel'): Promise<void> {
		if (action === 'dismiss') {
			dismissed.value = new Set([...dismissed.value, id])
			await operationsApi.dismiss(serverId, id).catch(() => undefined)
			return
		}
		try {
			await operationsApi.cancel(serverId, id)
		} catch (failure) {
			notifyFailure(messages.cancelFailed, failure)
		}
	}

	const canWriteFiles = computed(() =>
		hasServerPermission(toValue(options.permissions), 'FILES_WRITE'),
	)
	const busyReasons = computed<BusyReason[]>(() =>
		busyCodes.value.map((code) => ({ reason: BUSY_MESSAGES[code] ?? messages.backgroundTask })),
	)
	const isBusy = computed(() => busyReasons.value.length > 0 || !canWriteFiles.value)
	const busyTooltip = computed(() => {
		if (!canWriteFiles.value) return formatMessage(commonMessages.noPermissionAction)
		const [first] = busyReasons.value
		return first ? formatMessage(first.reason) : undefined
	})
	const busyWarning = computed(() => {
		const [first] = busyReasons.value
		return first ? formatMessage(first.reason) : null
	})

	onScopeDispose(
		socket.on('operations', (message) => {
			operations.value = message.operations
			busyCodes.value = message.busy_reasons
		}),
	)

	function offerDownload(path: string): void {
		const name = nameOf(path)
		addNotification({
			title: formatMessage(messages.zipReady),
			text: formatMessage(messages.zipReadyText, { file: name }),
			type: 'success',
			buttons: [
				{
					label: formatMessage(commonMessages.downloadButton),
					action: () => downloadFile(path, name),
				},
			],
		})
	}

	function settleZip(operation: Operation): void {
		if (!zipsStartedHere.delete(operation.id)) return
		if (operation.state === 'done' && operation.src) offerDownload(operation.src)
	}

	watch(operations, (next, before) => {
		const previous = new Map(before.map((operation) => [operation.id, operation.state]))
		const settled = next.filter(
			(operation) =>
				FILE_OPERATION_KINDS.has(operation.kind) &&
				TERMINAL_STATES.has(operation.state) &&
				(zipsStartedHere.has(operation.id) ||
					(previous.has(operation.id) && previous.get(operation.id) !== operation.state)),
		)
		if (settled.length === 0) return
		void refresh()
		for (const operation of settled) settleZip(operation)
	})

	const context: FileManagerContext = {
		items,
		loading,
		error,
		currentPath,
		navigateTo,
		editingFile,
		startEditing,
		stopEditing,
		createItem,
		renameItem,
		moveItem,
		deleteItem,
		readFile,
		readFileAsBlob,
		writeFile,
		downloadFile,
		statFile,
		zipFolder,
		zipPaths,
		uploadFiles,
		cancelUpload,
		uploadState,
		refresh,
		isBusy,
		busyTooltip,
		busyWarning,
		extractFile,
		activeOperations,
		dismissOperation,
		prefetchDirectory,
		prefetchFile,
		basePath,
	}
	provideFileManager(context)

	return {
		context,
		basePath,
		pending: useReadyState(directory),
		refresh,
		uploadState,
		cancelUpload,
		activeOperations,
		dismissOperation,
		busyCodes,
		busyReasons,
		canWriteFiles,
	}
}
