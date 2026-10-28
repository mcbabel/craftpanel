import type { AbstractModrinthClient, Archon, UploadHandle } from '@modrinth/api-client'
import { injectModrinthClient, provideModrinthClient } from '@modrinth/ui'

import {
	api,
	type Backup,
	type BackupOperation,
	type BackupSchedule,
	isApiRequestError,
	type Ulid,
} from '@/api'

// Modrinth's backups settings tab offers at most this many slots. A larger quota is shown as
// this window, and whatever lies beyond it stays with the manual backups.
const SETTINGS_SLOT_WINDOW = 15

function rethrow(cause: unknown): never {
	if (isApiRequestError(cause)) throw new Error(`${cause.status} ${cause.code}: ${cause.message}`)
	throw cause
}

function toQueueOperation(operation: BackupOperation): Archon.BackupsQueue.v1.BackupQueueOperation {
	return { ...operation, operation_id: operation.operation_id as unknown as number }
}

function toQueueBackup(backup: Backup): Archon.BackupsQueue.v1.BackupQueueBackup {
	return { ...backup, history: backup.history.map(toQueueOperation) }
}

export function toOptions(schedule: BackupSchedule): Archon.BackupsQueue.v1.WorldBackupOptions {
	const window = Math.min(schedule.limit, SETTINGS_SLOT_WINDOW)
	const automatic = Math.min(schedule.automatic_slots, window)
	return {
		limit: window,
		split_automatic: automatic,
		split_manual: window - automatic,
		interval: schedule.interval_hours,
	}
}

const ICON_PATH = '/server-icon.png'

class NotKept extends Error {
	readonly statusCode = 404
}

function missingOrRethrow(cause: unknown): never {
	if (isApiRequestError(cause) && cause.status === 404) throw new NotKept(cause.message)
	rethrow(cause)
}

// The vendored icon editor works on files next to the server. The panel keeps only the icon
// itself; the original upload and the editor settings Modrinth stores beside it are not kept.
function buildIconFiles(serverId: Ulid) {
	return {
		downloadFileWithAuth: async (_auth: unknown, path: string): Promise<Blob> => {
			if (path !== ICON_PATH) throw new NotKept(path)
			return await api.servers.icon(serverId).catch(missingOrRethrow)
		},
		uploadFileWithAuth: (_auth: unknown, path: string, file: Blob): UploadHandle<void> => {
			const promise =
				path === ICON_PATH ? api.servers.setIcon(serverId, file).catch(rethrow) : Promise.resolve()
			const handle: UploadHandle<void> = {
				promise,
				onProgress: () => handle,
				cancel: () => undefined,
			}
			return handle
		},
		deleteFileOrFolderWithAuth: async (_auth: unknown, path: string): Promise<void> => {
			if (path !== ICON_PATH) throw new NotKept(path)
			await api.servers.removeIcon(serverId).catch(missingOrRethrow)
		},
	}
}

function buildModules(serverId: Ulid) {
	async function listQueue(): Promise<Archon.BackupsQueue.v1.BackupsQueueResponse> {
		const page = await api.backups.list(serverId).catch(rethrow)
		return {
			active_operations: page.active_operations.map((operation) => ({
				...operation,
				operation_id: operation.operation_id as unknown as number,
			})),
			backups: page.backups.map(toQueueBackup),
		}
	}

	const backups_queue_v1 = {
		list: listQueue,
		listWithOptions: async (): Promise<Archon.BackupsQueue.v1.BackupsQueueWithOptionsResponse> => {
			const [queue, schedule] = await Promise.all([
				listQueue(),
				api.backups.schedule(serverId).catch(rethrow),
			])
			return {
				...queue,
				options: toOptions(schedule),
				next_auto_backup_at: schedule.automatic_slots > 0 ? schedule.next_run_at : null,
			}
		},
		patchOptions: async (
			_serverId: string,
			_worldId: string,
			patch: Archon.BackupsQueue.v1.PatchWorldBackupOptions,
			acknowledgeRotation = false,
		): Promise<Archon.BackupsQueue.v1.PatchWorldBackupOptionsResponse> => {
			const update = await api.backups
				.setSchedule(
					serverId,
					{ automatic_slots: patch.split_automatic, interval_hours: patch.interval },
					acknowledgeRotation ? { acknowledge_rotation: true } : {},
				)
				.catch(rethrow)
			return { applied: update.applied, backup_ids: update.backup_ids }
		},
		create: (_serverId: string, _worldId: string, request: { name: string }) =>
			api.backups.create(serverId, request).catch(rethrow),
		delete: (_serverId: string, _worldId: string, backupId: string) =>
			api.backups.remove(serverId, backupId).catch(rethrow),
		deleteMany: (_serverId: string, _worldId: string, backupIds: string[]) =>
			api.backups.bulkDelete(serverId, { backup_ids: backupIds }).catch(rethrow),
		restore: (_serverId: string, _worldId: string, backupId: string, request: { name: string }) =>
			api.backups.restore(serverId, backupId, request).catch(rethrow),
		retry: (_serverId: string, _worldId: string, backupId: string) =>
			api.backups.retry(serverId, backupId).catch(rethrow),
		ackCreate: (_serverId: string, _worldId: string, operationId: string) =>
			api.operations.dismiss(serverId, operationId).catch(rethrow),
		ackRestore: (_serverId: string, _worldId: string, operationId: string) =>
			api.operations.dismiss(serverId, operationId).catch(rethrow),
		cancelCreate: (_serverId: string, _worldId: string, operationId: string) =>
			api.operations.cancel(serverId, operationId).catch(rethrow),
		cancelRestore: (_serverId: string, _worldId: string, operationId: string) =>
			api.operations.cancel(serverId, operationId).catch(rethrow),
	}

	const backups_v1 = {
		rename: (_serverId: string, _worldId: string, backupId: string, request: { name: string }) =>
			api.backups.rename(serverId, backupId, request).catch(rethrow),
		delete: (_serverId: string, _worldId: string, backupId: string) =>
			api.backups.remove(serverId, backupId).catch(rethrow),
	}

	const properties_v1 = {
		getProperties: () => api.settings.properties(serverId).catch(rethrow),
		patchProperties: (
			_serverId: string,
			_worldId: string,
			patch: Archon.Content.v1.PatchPropertiesFields,
		) => api.settings.patchProperties(serverId, patch).catch(rethrow),
	}

	return { backups_queue_v1, backups_v1, properties_v1 }
}

function withFilesystemAuth(real: AbstractModrinthClient) {
	return Object.assign(Object.create(real.archon.servers_v0), {
		getFilesystemAuth: async (): Promise<Archon.Servers.v0.JWTAuth> => ({ url: '', token: '' }),
	})
}

export function provideArchonAdapters(serverId: Ulid): void {
	const real = injectModrinthClient()
	const shim: AbstractModrinthClient = Object.create(real)
	Object.defineProperty(shim, 'archon', {
		value: { ...real.archon, ...buildModules(serverId), servers_v0: withFilesystemAuth(real) },
		enumerable: true,
	})
	Object.defineProperty(shim, 'kyros', {
		value: { ...real.kyros, files_v0: buildIconFiles(serverId) },
		enumerable: true,
	})
	provideModrinthClient(shim)
}
