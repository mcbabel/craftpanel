<template>
	<div class="flex flex-col gap-6">
		<div class="flex flex-wrap items-end gap-3">
			<div class="flex flex-col gap-1">
				<h2 class="m-0 text-2xl font-extrabold text-contrast">{{ formatMessage(messages.title) }}</h2>
				<span class="text-secondary">
					{{ stats ? formatMessage(messages.subtitle, { version: stats.host.version, uptime: duration(stats.host.uptime_seconds) }) : '' }}
				</span>
			</div>
			<span v-if="stats" class="ml-auto text-sm text-secondary">
				{{ formatMessage(messages.refreshed, { time: formatTime(stats.measured_at) }) }}
			</span>
		</div>

		<Admonition v-if="failure" type="critical" :header="formatMessage(messages.failed)" :body="failure" />
		<LoadingIndicator v-else-if="!stats" />

		<template v-else>
			<div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
				<div v-for="tile in tiles" :key="tile.key" class="flex flex-col gap-3 rounded-2xl border border-solid border-surface-4 bg-bg-raised p-5">
					<div class="flex items-center gap-2 text-secondary">
						<component :is="tile.icon" aria-hidden="true" class="size-5" />
						<span class="font-semibold">{{ tile.label }}</span>
					</div>
					<span class="text-3xl font-extrabold text-contrast tabular-nums">{{ tile.value }}</span>
					<div v-if="tile.share !== null" class="h-2 overflow-hidden rounded-full bg-surface-4" role="presentation">
						<div class="h-full rounded-full" :class="barColor(tile.share)" :style="{ width: `${Math.min(100, tile.share * 100)}%` }" />
					</div>
					<span class="text-sm text-secondary">{{ tile.detail }}</span>
				</div>
			</div>

			<div class="grid gap-4 lg:grid-cols-2">
				<div class="flex flex-col gap-4 rounded-2xl border border-solid border-surface-4 bg-bg-raised p-5">
					<h3 class="m-0 text-lg font-bold text-contrast">{{ formatMessage(messages.memoryPlan) }}</h3>
					<div v-for="row in memoryRows" :key="row.label" class="flex flex-col gap-1.5">
						<div class="flex items-baseline gap-2">
							<span class="font-semibold text-primary">{{ row.label }}</span>
							<span class="ml-auto text-sm tabular-nums text-secondary">{{ row.text }}</span>
						</div>
						<div class="h-2 overflow-hidden rounded-full bg-surface-4" role="presentation">
							<div class="h-full rounded-full" :class="barColor(row.share)" :style="{ width: `${Math.min(100, row.share * 100)}%` }" />
						</div>
					</div>
					<span class="text-sm text-secondary">{{ formatMessage(messages.memoryPlanHint) }}</span>
				</div>

				<div class="flex flex-col gap-3 rounded-2xl border border-solid border-surface-4 bg-bg-raised p-5">
					<h3 class="m-0 text-lg font-bold text-contrast">{{ formatMessage(messages.panel) }}</h3>
					<dl class="m-0 grid grid-cols-2 gap-x-4 gap-y-3">
						<template v-for="fact in facts" :key="fact.label">
							<dt class="text-secondary">{{ fact.label }}</dt>
							<dd class="m-0 text-right font-semibold tabular-nums text-contrast">{{ fact.value }}</dd>
						</template>
					</dl>
				</div>
			</div>

			<section class="flex flex-col gap-3">
				<h3 class="m-0 text-lg font-bold text-contrast">
					{{ formatMessage(messages.runningTitle, { count: stats.running.length }) }}
				</h3>
				<p v-if="stats.running.length === 0" class="m-0 text-secondary">{{ formatMessage(messages.noneRunning) }}</p>
				<div v-else class="overflow-x-auto rounded-2xl border border-solid border-surface-4">
					<table class="w-full min-w-[44rem] border-collapse text-left">
						<thead class="bg-surface-2 text-sm text-secondary">
							<tr>
								<th class="px-4 py-3 font-semibold">{{ formatMessage(messages.colServer) }}</th>
								<th class="px-4 py-3 font-semibold">{{ formatMessage(messages.colOwner) }}</th>
								<th class="px-4 py-3 font-semibold">{{ formatMessage(messages.colMemory) }}</th>
								<th class="px-4 py-3 text-right font-semibold">{{ formatMessage(messages.colCpu) }}</th>
								<th class="px-4 py-3 text-right font-semibold">{{ formatMessage(messages.colUptime) }}</th>
							</tr>
						</thead>
						<tbody>
							<tr v-for="server in stats.running" :key="server.id" class="border-0 border-t border-solid border-surface-4 bg-bg-raised">
								<td class="px-4 py-3">
									<RouterLink :to="{ name: 'server-overview', params: { id: server.id } }" class="font-semibold text-contrast hover:underline">
										{{ server.name }}
									</RouterLink>
									<div class="text-sm text-secondary">
										{{ [loaderName(server.loader), server.game_version].filter(Boolean).join(' ') }}
										<template v-if="server.port"> · {{ formatMessage(messages.port, { port: server.port }) }}</template>
									</div>
								</td>
								<td class="px-4 py-3 text-primary">{{ server.owner.username }}</td>
								<td class="px-4 py-3">
									<div class="flex flex-col gap-1">
										<span class="text-sm tabular-nums text-primary">
											{{ formatBytes(server.ram_usage_bytes, 1) }} / {{ formatBytes(server.memory_mib * MIB, 0) }}
										</span>
										<div class="h-1.5 w-32 overflow-hidden rounded-full bg-surface-4" role="presentation">
											<div class="h-full rounded-full" :class="barColor(share(server.ram_usage_bytes, server.memory_mib * MIB))" :style="{ width: `${Math.min(100, share(server.ram_usage_bytes, server.memory_mib * MIB) * 100)}%` }" />
										</div>
									</div>
								</td>
								<td class="px-4 py-3 text-right tabular-nums text-primary">{{ server.cpu_percent.toFixed(1) }} %</td>
								<td class="px-4 py-3 text-right tabular-nums text-primary">
									{{ server.state === 'running' && server.running_since ? since(server.running_since) : formatMessage(powerLabels[server.state]) }}
								</td>
							</tr>
						</tbody>
					</table>
				</div>
			</section>

			<section class="flex flex-col gap-3">
				<h3 class="m-0 text-lg font-bold text-contrast">{{ formatMessage(messages.accountsTitle) }}</h3>
				<div class="overflow-x-auto rounded-2xl border border-solid border-surface-4">
					<table class="w-full min-w-[36rem] border-collapse text-left">
						<thead class="bg-surface-2 text-sm text-secondary">
							<tr>
								<th class="px-4 py-3 font-semibold">{{ formatMessage(messages.colUser) }}</th>
								<th class="px-4 py-3 text-right font-semibold">{{ formatMessage(messages.colServers) }}</th>
								<th class="px-4 py-3 font-semibold">{{ formatMessage(messages.colMemory) }}</th>
								<th class="px-4 py-3 text-right font-semibold">{{ formatMessage(messages.colCores) }}</th>
							</tr>
						</thead>
						<tbody>
							<tr v-for="account in stats.accounts" :key="account.user.id" class="border-0 border-t border-solid border-surface-4 bg-bg-raised">
								<td class="px-4 py-3 font-semibold text-contrast">{{ account.user.username }}</td>
								<td class="px-4 py-3 text-right tabular-nums text-primary">
									{{ formatMessage(messages.serversRunning, { running: account.running, total: account.servers }) }}
								</td>
								<td class="px-4 py-3 tabular-nums text-primary">
									{{ formatBytes(account.memory_bytes, 1) }}
									<span class="text-secondary">
										/ {{ account.memory_limit_mib === null ? formatMessage(messages.unlimited) : formatBytes(account.memory_limit_mib * MIB, 0) }}
									</span>
								</td>
								<td class="px-4 py-3 text-right tabular-nums text-primary">{{ account.cpu_cores.toFixed(2) }}</td>
							</tr>
						</tbody>
					</table>
				</div>
			</section>
		</template>
	</div>
</template>

<script setup lang="ts">
import { CpuIcon, DatabaseIcon, MemoryStickIcon, ServerIcon } from '@modrinth/assets'
import {
	Admonition,
	defineMessages,
	LoadingIndicator,
	type MessageDescriptor,
	useFormatBytes,
	useFormatDateTime,
	useVIntl,
} from '@modrinth/ui'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { RouterLink } from 'vue-router'

import { type AdminStatistics, api, isApiRequestError, type LoaderId, type PowerState } from '@/api'
import { LOADER_NAMES } from '@/providers/server-context'

const MIB = 1024 * 1024
const REFRESH_MS = 5_000

const { formatMessage } = useVIntl()
const formatBytes = useFormatBytes()
const formatTime = useFormatDateTime({ timeStyle: 'medium' })

const stats = ref<AdminStatistics | null>(null)
const failure = ref<string | null>(null)
let timer: ReturnType<typeof setInterval> | null = null

const messages = defineMessages({
	title: { id: 'panel.admin.stats.title', defaultMessage: 'Overview' },
	subtitle: {
		id: 'panel.admin.stats.subtitle',
		defaultMessage: 'CraftPanel {version} · machine up for {uptime}',
	},
	refreshed: { id: 'panel.admin.stats.refreshed', defaultMessage: 'Updated at {time}' },
	failed: { id: 'panel.admin.stats.failed', defaultMessage: 'The figures could not be read' },
	servers: { id: 'panel.admin.stats.servers', defaultMessage: 'Running servers' },
	serversDetail: {
		id: 'panel.admin.stats.servers-detail',
		defaultMessage: 'of {total} · {starting} starting · {crashed} crashed',
	},
	memory: { id: 'panel.admin.stats.memory', defaultMessage: 'Memory in use' },
	memoryDetail: {
		id: 'panel.admin.stats.memory-detail',
		defaultMessage: 'of {total} · game servers use {games}',
	},
	cpu: { id: 'panel.admin.stats.cpu', defaultMessage: 'Processor load' },
	cpuDetail: {
		id: 'panel.admin.stats.cpu-detail',
		defaultMessage: '{cores} cores · game servers use {used} cores',
	},
	disk: { id: 'panel.admin.stats.disk', defaultMessage: 'Disk in use' },
	diskDetail: {
		id: 'panel.admin.stats.disk-detail',
		defaultMessage: 'of {total} · {free} free',
	},
	memoryPlan: { id: 'panel.admin.stats.memory-plan', defaultMessage: 'Memory handed out' },
	promised: { id: 'panel.admin.stats.promised', defaultMessage: 'Given to all servers' },
	runningMemory: { id: 'panel.admin.stats.running-memory', defaultMessage: 'Given to running servers' },
	ofAssignable: { id: 'panel.admin.stats.of-assignable', defaultMessage: '{used} of {total}' },
	memoryPlanHint: {
		id: 'panel.admin.stats.memory-plan-hint',
		defaultMessage:
			'Measured against the memory the panel may hand out, which is the machine’s memory minus a reserve for the system.',
	},
	panel: { id: 'panel.admin.stats.panel', defaultMessage: 'Panel' },
	users: { id: 'panel.admin.stats.users', defaultMessage: 'Users' },
	stopped: { id: 'panel.admin.stats.stopped', defaultMessage: 'Stopped servers' },
	broken: { id: 'panel.admin.stats.broken', defaultMessage: 'Broken or installing' },
	operations: { id: 'panel.admin.stats.operations', defaultMessage: 'Tasks running' },
	backups: { id: 'panel.admin.stats.backups', defaultMessage: 'Backups' },
	backupsValue: { id: 'panel.admin.stats.backups-value', defaultMessage: '{count} · {size}' },
	processes: { id: 'panel.admin.stats.processes', defaultMessage: 'Game processes' },
	load: { id: 'panel.admin.stats.load', defaultMessage: 'Load (1, 5, 15 min)' },
	runningTitle: {
		id: 'panel.admin.stats.running-title',
		defaultMessage: 'Running now ({count})',
	},
	noneRunning: { id: 'panel.admin.stats.none-running', defaultMessage: 'No server is running.' },
	accountsTitle: { id: 'panel.admin.stats.accounts-title', defaultMessage: 'By account' },
	colServer: { id: 'panel.admin.stats.col-server', defaultMessage: 'Server' },
	colOwner: { id: 'panel.admin.stats.col-owner', defaultMessage: 'Owner' },
	colMemory: { id: 'panel.admin.stats.col-memory', defaultMessage: 'Memory' },
	colCpu: { id: 'panel.admin.stats.col-cpu', defaultMessage: 'CPU' },
	colUptime: { id: 'panel.admin.stats.col-uptime', defaultMessage: 'Up for' },
	colUser: { id: 'panel.admin.stats.col-user', defaultMessage: 'User' },
	colServers: { id: 'panel.admin.stats.col-servers', defaultMessage: 'Servers' },
	colCores: { id: 'panel.admin.stats.col-cores', defaultMessage: 'Cores in use' },
	serversRunning: {
		id: 'panel.admin.stats.servers-running',
		defaultMessage: '{running} of {total} running',
	},
	unlimited: { id: 'panel.admin.stats.unlimited', defaultMessage: 'no limit' },
	port: { id: 'panel.admin.stats.port', defaultMessage: 'port {port}' },
})

const powerLabels: Record<PowerState, MessageDescriptor> = defineMessages({
	stopped: { id: 'panel.server.power.stopped', defaultMessage: 'Stopped' },
	starting: { id: 'panel.server.power.starting', defaultMessage: 'Starting' },
	running: { id: 'panel.server.power.running', defaultMessage: 'Running' },
	stopping: { id: 'panel.server.power.stopping', defaultMessage: 'Stopping' },
	crashed: { id: 'panel.server.power.crashed', defaultMessage: 'Crashed' },
})

function share(used: number, total: number): number {
	return total > 0 ? used / total : 0
}

function barColor(fraction: number): string {
	if (fraction >= 0.9) return 'bg-red'
	if (fraction >= 0.75) return 'bg-orange'
	return 'bg-brand'
}

function duration(seconds: number): string {
	const days = Math.floor(seconds / 86_400)
	const hours = Math.floor((seconds % 86_400) / 3_600)
	const minutes = Math.floor((seconds % 3_600) / 60)
	if (days > 0) return `${days}d ${hours}h`
	if (hours > 0) return `${hours}h ${minutes}m`
	return `${minutes}m`
}

function since(stamp: string): string {
	return duration(Math.max(0, Math.floor((Date.now() - Date.parse(stamp)) / 1000)))
}

function loaderName(loader: LoaderId | null): string {
	return loader ? (LOADER_NAMES[loader] ?? loader) : ''
}

const tiles = computed(() => {
	const value = stats.value
	if (!value) return []
	const memoryUsed = value.host.memory_total_bytes - value.host.memory_available_bytes
	const diskUsed = value.host.disk_total_bytes - value.host.disk_free_bytes
	const load = value.host.load_average[0]
	return [
		{
			key: 'servers',
			icon: ServerIcon,
			label: formatMessage(messages.servers),
			value: String(value.servers.running),
			share: share(value.servers.running, value.servers.total),
			detail: formatMessage(messages.serversDetail, {
				total: value.servers.total,
				starting: value.servers.starting,
				crashed: value.servers.crashed,
			}),
		},
		{
			key: 'memory',
			icon: MemoryStickIcon,
			label: formatMessage(messages.memory),
			value: formatBytes(memoryUsed, 1),
			share: share(memoryUsed, value.host.memory_total_bytes),
			detail: formatMessage(messages.memoryDetail, {
				total: formatBytes(value.host.memory_total_bytes, 0),
				games: formatBytes(value.games.memory_bytes, 1),
			}),
		},
		{
			key: 'cpu',
			icon: CpuIcon,
			label: formatMessage(messages.cpu),
			value: `${Math.round(share(load, value.host.cpu_cores) * 100)} %`,
			share: share(load, value.host.cpu_cores),
			detail: formatMessage(messages.cpuDetail, {
				cores: value.host.cpu_cores,
				used: value.games.cpu_cores.toFixed(2),
			}),
		},
		{
			key: 'disk',
			icon: DatabaseIcon,
			label: formatMessage(messages.disk),
			value: formatBytes(diskUsed, 1),
			share: share(diskUsed, value.host.disk_total_bytes),
			detail: formatMessage(messages.diskDetail, {
				total: formatBytes(value.host.disk_total_bytes, 0),
				free: formatBytes(value.host.disk_free_bytes, 0),
			}),
		},
	]
})

const memoryRows = computed(() => {
	const value = stats.value
	if (!value) return []
	const total = value.memory.assignable_mib
	return [
		{ label: formatMessage(messages.promised), mib: value.memory.promised_mib },
		{ label: formatMessage(messages.runningMemory), mib: value.memory.running_mib },
	].map((row) => ({
		label: row.label,
		share: share(row.mib, total),
		text: formatMessage(messages.ofAssignable, {
			used: formatBytes(row.mib * MIB, 1),
			total: formatBytes(total * MIB, 1),
		}),
	}))
})

const facts = computed(() => {
	const value = stats.value
	if (!value) return []
	return [
		{ label: formatMessage(messages.users), value: String(value.users) },
		{ label: formatMessage(messages.stopped), value: String(value.servers.stopped) },
		{ label: formatMessage(messages.broken), value: String(value.servers.broken + value.servers.installing) },
		{ label: formatMessage(messages.operations), value: String(value.active_operations) },
		{
			label: formatMessage(messages.backups),
			value: formatMessage(messages.backupsValue, {
				count: value.backups.count,
				size: formatBytes(value.backups.bytes, 1),
			}),
		},
		{ label: formatMessage(messages.processes), value: String(value.games.pids) },
		{ label: formatMessage(messages.load), value: value.host.load_average.map((entry) => entry.toFixed(2)).join(' · ') },
	]
})

async function load(): Promise<void> {
	try {
		stats.value = await api.admin.statistics()
		failure.value = null
	} catch (error) {
		if (!stats.value) failure.value = isApiRequestError(error) ? error.message : String(error)
	}
}

onMounted(() => {
	void load()
	timer = setInterval(() => {
		if (document.visibilityState === 'visible') void load()
	}, REFRESH_MS)
})

onBeforeUnmount(() => {
	if (timer !== null) clearInterval(timer)
})
</script>
