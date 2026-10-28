import type { Labrinth, LauncherMeta, Paper, Purpur } from '@modrinth/api-client'
import { computed, type ComputedRef, ref, type Ref } from 'vue'
import type { HistoryState } from 'vue-router'

import {
	api,
	type GameVersionEntry,
	type LoaderBuild,
	type LoaderId,
	type LoaderInfo,
	type PropertiesFields,
} from '@/api'

export interface ModpackPick {
	projectId: string
	versionId?: string | null
	name: string
	iconUrl?: string
}

export type SetupChoice =
	| {
			kind: 'loader'
			loader: LoaderId
			gameVersion: string
			loaderVersion: string | null
			properties: PropertiesFields
	  }
	| {
			kind: 'modpack'
			projectId: string
			versionId: string
			name: string
			properties: PropertiesFields
	  }
	| { kind: 'upload'; file: File; properties: PropertiesFields }

// A picked pack travels in the navigation's history state rather than the address, so only a
// click inside the panel can open the setup with it — never a link from elsewhere.
const STATE_KEY = 'modpack'

function text(value: unknown): string | null {
	return typeof value === 'string' && value.length > 0 ? value : null
}

export function modpackState(pick: ModpackPick): HistoryState {
	return { [STATE_KEY]: { ...pick, versionId: pick.versionId ?? null } }
}

export function modpackFromState(state: unknown): ModpackPick | null {
	if (typeof state !== 'object' || state === null) return null
	const raw = (state as Record<string, unknown>)[STATE_KEY]
	if (typeof raw !== 'object' || raw === null) return null
	const fields = raw as Record<string, unknown>
	const projectId = text(fields.projectId)
	if (projectId === null) return null
	return {
		projectId,
		versionId: text(fields.versionId),
		name: text(fields.name) ?? projectId,
		iconUrl: text(fields.iconUrl) ?? undefined,
	}
}

// Takes the pick out of the current history entry, so a reload or a step back does not open
// the setup a second time.
export function takeModpackPick(): ModpackPick | null {
	const state: unknown = window.history.state
	const pick = modpackFromState(state)
	if (pick !== null) {
		const rest = { ...(state as Record<string, unknown>) }
		delete rest[STATE_KEY]
		window.history.replaceState(rest, '')
	}
	return pick
}

const SUPPORTED_WAVE = 1
const PLACEHOLDER = '${modrinth.gameVersion}'
const SHIMMED = new Set<string>(['paper', 'purpur'])
const SAME_BUILDS_FOR_EVERY_VERSION = new Set<string>(['fabric', 'quilt'])

export type CatalogClient = Pick<typeof api.settings, 'loaders' | 'gameVersions' | 'builds'>

export interface PaperShim {
	versions_v3: {
		getProject: () => Promise<Paper.Versions.v3.Project>
		getBuilds: (mcVersion: string) => Promise<Paper.Versions.v3.VersionBuilds>
	}
}

export interface PurpurShim {
	versions_v2: {
		getProject: () => Promise<Purpur.Versions.v2.Project>
		getBuilds: (mcVersion: string) => Promise<Purpur.Versions.v2.VersionBuilds>
	}
}

export interface CreationCatalog {
	loaders: Ref<LoaderInfo[]>
	ready: Ref<boolean>
	failure: Ref<unknown>
	flowLoaders: ComputedRef<string[]>
	modpackLoaders: ComputedRef<string[]>
	gameVersionTags: ComputedRef<Labrinth.Tags.v2.GameVersion[]>
	load: () => Promise<void>
	manifest: (loader: string) => Promise<LauncherMeta.Manifest.v0.Manifest>
	paper: PaperShim
	purpur: PurpurShim
}

export function toVersionTags(lists: GameVersionEntry[][]): Labrinth.Tags.v2.GameVersion[] {
	const seen = new Set<string>()
	const tags: Labrinth.Tags.v2.GameVersion[] = []
	for (const list of lists) {
		for (const entry of list) {
			if (seen.has(entry.version)) continue
			seen.add(entry.version)
			tags.push({ version: entry.version, version_type: entry.version_type, date: '', major: false })
		}
	}
	return tags
}

function loaderEntry(build: LoaderBuild): LauncherMeta.Manifest.v0.LoaderVersion {
	return { id: build.id, url: '', stable: build.stable }
}

export function sharedManifest(
	versions: GameVersionEntry[],
	builds: LoaderBuild[],
): LauncherMeta.Manifest.v0.Manifest {
	return {
		gameVersions: [
			{ id: PLACEHOLDER, stable: true, loaders: builds.map(loaderEntry) },
			...versions.map((entry) => ({
				id: entry.version,
				stable: entry.version_type === 'release',
				loaders: [],
			})),
		],
	}
}

export function perVersionManifest(
	versions: GameVersionEntry[],
	builds: Map<string, LoaderBuild[]>,
): LauncherMeta.Manifest.v0.Manifest {
	return {
		gameVersions: versions.map((entry) => ({
			id: entry.version,
			stable: entry.version_type === 'release',
			loaders: (builds.get(entry.version) ?? []).map(loaderEntry),
		})),
	}
}

export function paperBuild(build: LoaderBuild): Paper.Versions.v3.Build {
	return {
		id: Number(build.id),
		time: build.released ?? '',
		channel: build.channel_tag ?? (build.stable ? 'STABLE' : 'BETA'),
	}
}

export function useCreationCatalog(client: CatalogClient = api.settings): CreationCatalog {
	const loaders = ref<LoaderInfo[]>([])
	const versions = ref(new Map<string, GameVersionEntry[]>())
	const ready = ref(false)
	const failure = ref<unknown>(null)
	const builds = new Map<string, Promise<LoaderBuild[]>>()

	const usable = computed(() => loaders.value.filter((loader) => loader.wave <= SUPPORTED_WAVE))
	const flowLoaders = computed(() =>
		usable.value.filter((loader) => loader.id !== 'vanilla').map((loader) => loader.id),
	)
	const modpackLoaders = computed(() =>
		usable.value.filter((loader) => loader.kind === 'modloader').map((loader) => loader.id),
	)

	const gameVersionTags = computed(() => {
		const ordered = ['vanilla', ...flowLoaders.value]
		return toVersionTags(ordered.map((id) => versions.value.get(id) ?? []))
	})

	function versionsOf(loader: string): GameVersionEntry[] {
		return versions.value.get(loader) ?? []
	}

	function buildsOf(loader: string, gameVersion: string): Promise<LoaderBuild[]> {
		const key = `${loader}\u0000${gameVersion}`
		let pending = builds.get(key)
		if (!pending) {
			pending = client
				.builds(loader as LoaderId, gameVersion)
				.then((list) => list.builds)
				.catch(() => [])
			builds.set(key, pending)
		}
		return pending
	}

	async function load(): Promise<void> {
		failure.value = null
		try {
			loaders.value = (await client.loaders()).loaders
			const wanted = usable.value.map((loader) => loader.id)
			const lists = await Promise.all(
				wanted.map((id) =>
					client.gameVersions(id).then(
						(list) => list.game_versions,
						() => [],
					),
				),
			)
			const next = new Map<string, GameVersionEntry[]>()
			wanted.forEach((id, index) => next.set(id, lists[index]))
			versions.value = next
			ready.value = true
		} catch (cause) {
			failure.value = cause
		}
	}

	async function manifest(loader: string): Promise<LauncherMeta.Manifest.v0.Manifest> {
		const id = loader === 'neo' ? 'neoforge' : loader
		const list = versionsOf(id)
		if (list.length === 0 || SHIMMED.has(id)) return { gameVersions: [] }
		if (SAME_BUILDS_FOR_EVERY_VERSION.has(id)) {
			return sharedManifest(list, await buildsOf(id, list[0].version))
		}
		const found = await Promise.all(list.map((entry) => buildsOf(id, entry.version)))
		return perVersionManifest(list, new Map(list.map((entry, index) => [entry.version, found[index]])))
	}

	const paper: PaperShim = {
		versions_v3: {
			getProject: async () => ({
				project: { id: 'paper', name: 'Paper' },
				versions: { all: versionsOf('paper').map((entry) => entry.version) },
			}),
			getBuilds: async (mcVersion) => ({
				builds: (await buildsOf('paper', mcVersion)).map(paperBuild),
			}),
		},
	}

	const purpur: PurpurShim = {
		versions_v2: {
			getProject: async () => ({
				project: 'purpur',
				versions: versionsOf('purpur').map((entry) => entry.version),
			}),
			getBuilds: async (mcVersion) => ({
				builds: { all: (await buildsOf('purpur', mcVersion)).map((build) => build.id) },
			}),
		},
	}

	return {
		loaders,
		ready,
		failure,
		flowLoaders,
		modpackLoaders,
		gameVersionTags,
		load,
		manifest,
		paper,
		purpur,
	}
}

export function modpackFilter(loaders: string[]): string {
	const environment =
		'(client_side = "optional" OR client_side = "required") AND server_side = "required"'
	const categories =
		loaders.length === 0
			? null
			: `(${loaders.map((loader) => `categories = "${loader}"`).join(' OR ')})`
	return ['project_types = "modpack"', categories, environment].filter(Boolean).join(' AND ')
}

export function runnableVersions<T extends { loaders: string[] }>(
	versions: T[],
	loaders: string[],
): T[] {
	return versions.filter(
		(version) =>
			version.loaders.length === 0 || version.loaders.some((loader) => loaders.includes(loader)),
	)
}
