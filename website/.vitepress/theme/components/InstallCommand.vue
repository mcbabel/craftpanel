<template>
	<div class="install">
		<div class="tabs" role="tablist" :aria-label="'Install method'">
			<button
				v-for="option in options"
				:key="option.id"
				role="tab"
				type="button"
				class="tab"
				:aria-selected="option.id === chosen"
				@click="chosen = option.id"
			>
				{{ option.label }}
			</button>
		</div>
		<div class="command" role="tabpanel">
			<pre><code><template v-for="(line, index) in current.lines" :key="index"><span class="prompt" aria-hidden="true">{{ line.root ? '#' : '$' }} </span>{{ line.text }}<span v-if="line.note" class="note">  # {{ line.note }}</span>
</template></code></pre>
			<button type="button" class="copy" :class="{ done: copied }" @click="copy">
				<svg v-if="!copied" viewBox="0 0 24 24" aria-hidden="true">
					<rect x="9" y="9" width="12" height="12" rx="2" />
					<path d="M5 15V5a2 2 0 0 1 2-2h10" />
				</svg>
				<svg v-else viewBox="0 0 24 24" aria-hidden="true"><path d="m5 12 5 5 9-10" /></svg>
				<span>{{ copied ? 'Copied' : 'Copy' }}</span>
			</button>
		</div>
		<p class="hint">{{ current.hint }}</p>
	</div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue'

const script = 'https://github.com/MCbabel/craftpanel/releases/latest/download/install.sh'

interface Line {
	text: string
	note?: string
	root?: boolean
}

const options: { id: string; label: string; lines: Line[]; hint: string }[] = [
	{
		id: 'quick',
		label: 'One line',
		lines: [{ text: `curl -fsSL ${script} | sudo bash` }],
		hint: 'Run it again later to update or uninstall.',
	},
	{
		id: 'careful',
		label: 'Check it first',
		lines: [
			{ text: `curl -fsSL -o install.sh ${script}` },
			{ text: 'sha256sum install.sh', note: 'compare with the release notes' },
			{ text: 'less install.sh' },
			{ text: 'sudo bash install.sh' },
		],
		hint: 'The script only defines functions until its last line, so a broken download runs nothing.',
	},
]

const chosen = ref(options[0].id)
const current = computed(() => options.find((option) => option.id === chosen.value) ?? options[0])

const copied = ref(false)
let reset: ReturnType<typeof setTimeout> | undefined

async function copy(): Promise<void> {
	const text = current.value.lines.map((line) => line.text).join('\n')
	try {
		await navigator.clipboard.writeText(text)
	} catch {
		return
	}
	copied.value = true
	clearTimeout(reset)
	reset = setTimeout(() => (copied.value = false), 1800)
}
</script>

<style scoped>
.install {
	width: 100%;
	max-width: 62rem;
}

.tabs {
	display: inline-flex;
	gap: 0.25rem;
	padding: 0.25rem;
	margin-bottom: 0.75rem;
	border-radius: 999px;
	background: var(--vp-c-bg-soft);
	border: 1px solid var(--vp-c-divider);
}

.tab {
	padding: 0.375rem 0.875rem;
	border-radius: 999px;
	font-size: 0.875rem;
	font-weight: 600;
	color: var(--vp-c-text-2);
	transition: background-color 0.15s, color 0.15s;
}

.tab:hover {
	color: var(--vp-c-text-1);
}

.tab[aria-selected='true'] {
	background: var(--vp-c-brand-soft);
	color: var(--vp-c-brand-1);
}

.tab:focus-visible,
.copy:focus-visible {
	outline: 2px solid var(--vp-c-brand-1);
	outline-offset: 2px;
}

.command {
	display: flex;
	align-items: flex-start;
	gap: 0.5rem;
	padding: 0.625rem;
	border-radius: 1rem;
	background: var(--cp-command-bg);
	border: 1px solid var(--cp-card-border);
	box-shadow: var(--cp-shadow);
}

.command pre {
	flex: 1;
	min-width: 0;
	margin: 0;
	padding: 0.5rem 0.625rem;
	overflow-x: auto;
	font-family: var(--vp-font-family-mono);
	font-size: 0.875rem;
	line-height: 1.75;
	color: var(--cp-command-text);
}

.prompt {
	color: var(--cp-green-400);
	user-select: none;
}

.note {
	color: #7b8692;
}

.copy {
	flex-shrink: 0;
	display: inline-flex;
	align-items: center;
	gap: 0.375rem;
	padding: 0.4rem 0.75rem;
	border-radius: 0.625rem;
	font-size: 0.8125rem;
	font-weight: 600;
	color: #e6eaee;
	background: rgba(255, 255, 255, 0.08);
	transition: background-color 0.15s, color 0.15s;
}

.copy:hover {
	background: rgba(255, 255, 255, 0.14);
}

.copy.done {
	color: var(--cp-green-400);
}

.copy svg {
	width: 1rem;
	height: 1rem;
	fill: none;
	stroke: currentColor;
	stroke-width: 2;
	stroke-linecap: round;
	stroke-linejoin: round;
}

.hint {
	margin-top: 0.75rem;
	font-size: 0.875rem;
	color: var(--vp-c-text-2);
}

@media (max-width: 640px) {
	.command {
		flex-direction: column;
		align-items: stretch;
	}

	.copy {
		justify-content: center;
	}

	.command pre {
		font-size: 0.8125rem;
	}
}
</style>
