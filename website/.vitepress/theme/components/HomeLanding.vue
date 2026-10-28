<template>
	<div class="landing">
		<section class="block hero wrap">
			<img class="mark" :src="withBase('/logo.svg')" alt="" width="56" height="56" />
			<h1>Minecraft servers on your own Linux machine.</h1>
			<p class="lede">
				CraftPanel is a self-hosted web panel for one machine. A single command installs it, every
				user gets an isolated system account with its own limits, and servers keep running while
				the panel updates.
			</p>
			<InstallCommand />
			<p class="requirements">
				Needs Linux with systemd and cgroup v2, on x86_64 or aarch64. Java is fetched for you.
			</p>
			<div class="actions">
				<a class="button primary" :href="withBase('/docs/installation')">Installation guide</a>
				<a class="button" :href="repository">Source on GitHub</a>
			</div>
		</section>

		<section class="block wrap">
			<figure class="screen">
				<div class="bar" aria-hidden="true"><span /><span /><span /></div>
				<img
					class="only-light"
					:src="withBase('/screenshots/overview-light.webp')"
					alt="The server overview in CraftPanel: CPU, memory and storage, the public address and the live console."
					width="2400"
					height="1500"
				/>
				<img
					class="only-dark"
					:src="withBase('/screenshots/overview-dark.webp')"
					alt="The server overview in CraftPanel: CPU, memory and storage, the public address and the live console."
					width="2400"
					height="1500"
				/>
			</figure>
		</section>

		<section class="block wrap split">
			<div>
				<h2>What the installer does</h2>
				<ol class="steps">
					<li>
						<strong>Asks three things.</strong> The web port, the port range for game servers and
						a name for the administrator. Every question has a default.
					</li>
					<li>
						<strong>Downloads and checks the release.</strong> The bundle for your architecture is
						verified against its published SHA-256 sum before anything is installed.
					</li>
					<li>
						<strong>Sets up two services.</strong> The panel runs as an unprivileged user; a
						small helper with five fixed commands is the only part that runs as root.
					</li>
					<li>
						<strong>Prints the password once.</strong> Sign in with it and choose your own.
					</li>
				</ol>
			</div>
			<div class="terminal" aria-label="Example installer run">
				<pre><code><span class="dim">Web interface port [8080]:</span>
<span class="dim">First port for Minecraft servers [25565]:</span>
<span class="dim">Last port for Minecraft servers [25700]:</span>
<span class="dim">Administrator username [admin]:</span> steve

  checksum verified: sha256 9f2c…e41a

<strong>The administrator account has been created.</strong>
  Username  steve
  Password  <span class="secret">••••••••••••••••</span>
<span class="dim">Write it down now.</span></code></pre>
			</div>
		</section>

		<section class="block wrap">
			<h2>Everything a server needs</h2>
			<div class="groups">
				<div v-for="group in groups" :key="group.title" class="group">
					<h3>{{ group.title }}</h3>
					<ul>
						<li v-for="item in group.items" :key="item">{{ item }}</li>
					</ul>
				</div>
			</div>
		</section>

		<section class="block wrap split reverse">
			<div>
				<h2>Mods and plugins from Modrinth</h2>
				<p>
					The interface is built on Modrinth's open-source UI, so searching, installing and
					updating content works the way it does on modrinth.com. The panel only offers what fits
					your server's loader and game version, and installs dependencies with it.
				</p>
				<p>
					Modpacks work the same way: start a server from a Modrinth pack or an uploaded
					<code>.mrpack</code>, and switch its version later.
				</p>
			</div>
			<figure class="screen small">
				<img
					class="only-light"
					loading="lazy"
					:src="withBase('/screenshots/browse-light.webp')"
					alt="Browsing plugins from Modrinth inside CraftPanel."
					width="2400"
					height="1500"
				/>
				<img
					class="only-dark"
					loading="lazy"
					:src="withBase('/screenshots/browse-dark.webp')"
					alt="Browsing plugins from Modrinth inside CraftPanel."
					width="2400"
					height="1500"
				/>
			</figure>
		</section>

		<section class="block wrap">
			<h2>Three parts, one machine</h2>
			<p class="intro">
				No containers, no message queue, no daemon cluster. Each part runs with only the rights it
				needs.
			</p>
			<div class="parts">
				<div class="part">
					<span class="who">user craftpanel</span>
					<h3>Panel</h3>
					<p>Web interface, API, database, files, backups. Never becomes root.</p>
				</div>
				<div class="link" aria-hidden="true" />
				<div class="part">
					<span class="who">root</span>
					<h3>Helper</h3>
					<p>
						Five commands over a Unix socket: create and remove accounts, set limits, start
						supervisors, fix ownership.
					</p>
				</div>
				<div class="link" aria-hidden="true" />
				<div class="part">
					<span class="who">user craft-&lt;id&gt;</span>
					<h3>Supervisor</h3>
					<p>One per server. Runs Java, keeps the console, and outlives panel updates.</p>
				</div>
			</div>
			<a class="more" :href="withBase('/docs/architecture')">How the parts talk to each other</a>
		</section>

		<section class="block wrap">
			<h2>Your own accounts, if you want them</h2>
			<p class="intro">
				Optional services connect to accounts you already have. Nothing is sent anywhere until you
				set it up.
			</p>
			<div class="services">
				<a class="service" :href="withBase('/docs/google-drive')">
					<h3>Google Drive</h3>
					<p>Each user can keep backup copies in their own Drive.</p>
				</a>
				<a class="service" :href="withBase('/docs/playit')">
					<h3>playit.gg</h3>
					<p>A public address per server, without port forwarding.</p>
				</a>
				<a class="service" :href="withBase('/docs/mail')">
					<h3>Mail</h3>
					<p>Sign-up verification and password resets, sent through your Resend account.</p>
				</a>
			</div>
		</section>

		<section class="block wrap closing">
			<h2>Free software</h2>
			<p>
				CraftPanel is licensed under the AGPL-3.0. Use it at home or commercially; if you offer a
				changed version to others, share its source.
			</p>
			<div class="actions">
				<a class="button primary" :href="withBase('/docs/')">Read the docs</a>
				<a class="button" :href="`${repository}/releases`">Releases</a>
			</div>
		</section>
	</div>
</template>

<script setup lang="ts">
import { withBase } from 'vitepress'

import InstallCommand from './InstallCommand.vue'

const repository = 'https://github.com/MCbabel/craftpanel'

const groups = [
	{
		title: 'Run servers',
		items: [
			'Vanilla, Paper, Folia, Purpur, Leaf, Fabric, Velocity, CraftBukkit and Limbo',
			'Live console with search and log sharing',
			'CPU, memory and storage at a glance',
			'The right Java runtime per Minecraft version',
		],
	},
	{
		title: 'Manage files',
		items: [
			'Browse, edit, upload and move files in the browser',
			'Pack folders into a ZIP, unpack archives',
			'Server properties as a form',
		],
	},
	{
		title: 'Keep it safe',
		items: [
			'Manual and scheduled backups that save the world first',
			'Memory, CPU, process and disk limits per user',
			'A separate system account for every panel user',
		],
	},
	{
		title: 'Share it',
		items: [
			'Owner, editor and limited roles per server',
			'Invitations, sign-up with approval, audit log',
			'English and German, on desktop and phone',
		],
	},
]
</script>

<style scoped>
.landing {
	padding-bottom: 6rem;
}

.wrap {
	max-width: 72rem;
	margin: 0 auto;
	padding: 0 1.5rem;
}

.block + .block {
	margin-top: 7rem;
}

h1 {
	max-width: 15ch;
	margin: 1.5rem 0 1.25rem;
	font-size: clamp(2.5rem, 6vw, 4.25rem);
	font-weight: 800;
	line-height: 1.04;
	letter-spacing: -0.035em;
	color: var(--vp-c-text-1);
}

h2 {
	margin: 0 0 1rem;
	font-size: clamp(1.75rem, 3.2vw, 2.25rem);
	font-weight: 800;
	line-height: 1.15;
	letter-spacing: -0.025em;
	color: var(--vp-c-text-1);
}

h3 {
	margin: 0 0 0.5rem;
	font-size: 1.0625rem;
	font-weight: 700;
	color: var(--vp-c-text-1);
}

p {
	color: var(--vp-c-text-2);
	line-height: 1.7;
}

p code {
	font-family: var(--vp-font-family-mono);
	font-size: 0.875em;
	padding: 0.125rem 0.375rem;
	border-radius: 0.375rem;
	background: var(--vp-c-bg-soft);
	color: var(--vp-c-text-1);
}

.hero {
	padding-top: 4.5rem;
}

.mark {
	display: block;
	width: 3.5rem;
	height: 3.5rem;
}

.lede {
	max-width: 38rem;
	margin: 0 0 2.25rem;
	font-size: 1.1875rem;
}

.requirements {
	margin: 0.25rem 0 0;
	font-size: 0.875rem;
	color: var(--vp-c-text-3);
}

.actions {
	display: flex;
	flex-wrap: wrap;
	gap: 0.75rem;
	margin-top: 2rem;
}

.button {
	display: inline-flex;
	align-items: center;
	padding: 0.625rem 1.25rem;
	border-radius: 0.875rem;
	font-weight: 700;
	font-size: 0.9375rem;
	color: var(--vp-c-text-1);
	background: var(--vp-c-bg-soft);
	border: 1px solid var(--vp-c-divider);
	transition: background-color 0.15s, border-color 0.15s;
}

.button:hover {
	border-color: var(--vp-c-text-3);
}

.button.primary {
	color: var(--vp-button-brand-text);
	background: var(--vp-button-brand-bg);
	border-color: transparent;
}

.button.primary:hover {
	background: var(--vp-button-brand-hover-bg);
}

.button:focus-visible,
.service:focus-visible,
.more:focus-visible {
	outline: 2px solid var(--vp-c-brand-1);
	outline-offset: 3px;
}

.screen {
	position: relative;
	margin: 0;
	border-radius: 1.25rem;
	overflow: hidden;
	background: var(--cp-card);
	border: 1px solid var(--cp-card-border);
	box-shadow: var(--cp-shadow);
}

.screen img {
	width: 100%;
	height: auto;
}

.bar {
	display: flex;
	gap: 0.4rem;
	padding: 0.75rem 1rem;
	border-bottom: 1px solid var(--cp-card-border);
}

.bar span {
	width: 0.625rem;
	height: 0.625rem;
	border-radius: 999px;
	background: var(--vp-c-divider);
}

.only-dark {
	display: none;
}

:global(.dark .landing .only-dark) {
	display: block;
}

:global(.dark .landing .only-light) {
	display: none;
}

.split {
	display: grid;
	grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
	gap: 3.5rem;
	align-items: center;
}

.split p + p {
	margin-top: 1rem;
}

.steps {
	margin: 1.5rem 0 0;
	padding: 0;
	list-style: none;
	counter-reset: step;
}

.steps li {
	position: relative;
	padding-left: 2.75rem;
	color: var(--vp-c-text-2);
	line-height: 1.65;
	counter-increment: step;
}

.steps li + li {
	margin-top: 1.25rem;
}

.steps li::before {
	content: counter(step);
	position: absolute;
	left: 0;
	top: 0;
	display: grid;
	place-items: center;
	width: 1.75rem;
	height: 1.75rem;
	border-radius: 0.5rem;
	font-size: 0.875rem;
	font-weight: 800;
	color: var(--vp-c-brand-1);
	background: var(--vp-c-brand-soft);
}

.steps strong {
	color: var(--vp-c-text-1);
}

.terminal {
	border-radius: 1.25rem;
	background: var(--cp-command-bg);
	border: 1px solid var(--cp-card-border);
	box-shadow: var(--cp-shadow);
}

.terminal pre {
	margin: 0;
	padding: 1.5rem;
	overflow-x: auto;
	font-family: var(--vp-font-family-mono);
	font-size: 0.8125rem;
	line-height: 1.8;
	color: var(--cp-command-text);
}

.terminal .dim {
	color: #8a949e;
}

.terminal strong {
	color: #ffffff;
}

.terminal .secret {
	color: var(--cp-green-400);
}

.groups {
	display: grid;
	grid-template-columns: repeat(4, minmax(0, 1fr));
	gap: 2.5rem;
	margin-top: 2.5rem;
}

.group {
	padding-top: 1.25rem;
	border-top: 2px solid var(--vp-c-brand-1);
}

.group ul {
	margin: 0.75rem 0 0;
	padding: 0;
	list-style: none;
}

.group li {
	color: var(--vp-c-text-2);
	line-height: 1.55;
}

.group li + li {
	margin-top: 0.625rem;
}

.reverse > figure {
	order: -1;
}

.intro {
	max-width: 38rem;
	margin: 0;
}

.parts {
	display: grid;
	grid-template-columns: minmax(0, 1fr) 2.5rem minmax(0, 1fr) 2.5rem minmax(0, 1fr);
	align-items: stretch;
	margin-top: 2.5rem;
}

.part {
	padding: 1.5rem;
	border-radius: 1.25rem;
	background: var(--cp-card);
	border: 1px solid var(--cp-card-border);
}

.part:nth-child(3) {
	border-color: var(--vp-c-brand-1);
}

.part p {
	margin: 0;
	font-size: 0.9375rem;
	line-height: 1.6;
}

.who {
	display: inline-block;
	margin-bottom: 0.875rem;
	padding: 0.125rem 0.5rem;
	border-radius: 0.375rem;
	font-family: var(--vp-font-family-mono);
	font-size: 0.75rem;
	color: var(--vp-c-text-2);
	background: var(--vp-c-bg-soft);
}

.part:nth-child(3) .who {
	color: var(--vp-c-brand-1);
	background: var(--vp-c-brand-soft);
}

.link {
	align-self: center;
	height: 2px;
	background: var(--vp-c-divider);
}

.more {
	display: inline-block;
	margin-top: 1.5rem;
	font-weight: 600;
	color: var(--vp-c-brand-1);
	text-underline-offset: 3px;
}

.more:hover {
	text-decoration: underline;
}

.services {
	display: grid;
	grid-template-columns: repeat(3, minmax(0, 1fr));
	gap: 1rem;
	margin-top: 2rem;
}

.service {
	display: block;
	padding: 1.25rem 1.5rem;
	border-radius: 1rem;
	border: 1px solid var(--cp-card-border);
	transition: border-color 0.15s;
}

.service:hover {
	border-color: var(--vp-c-brand-1);
}

.service p {
	margin: 0;
	font-size: 0.9375rem;
	line-height: 1.6;
}

.closing p {
	max-width: 36rem;
	margin: 0;
}

@media (max-width: 960px) {
	.split {
		grid-template-columns: minmax(0, 1fr);
		gap: 2rem;
	}

	.reverse > figure {
		order: 0;
	}

	.groups {
		grid-template-columns: repeat(2, minmax(0, 1fr));
	}

	.parts {
		grid-template-columns: minmax(0, 1fr);
	}

	.link {
		justify-self: center;
		width: 2px;
		height: 1.5rem;
	}

	.services {
		grid-template-columns: minmax(0, 1fr);
	}
}

@media (max-width: 640px) {
	.wrap {
		padding: 0 1rem;
	}

	.block + .block {
		margin-top: 4.5rem;
	}

	.hero {
		padding-top: 2.5rem;
	}

	.lede {
		font-size: 1.0625rem;
	}

	.groups {
		grid-template-columns: minmax(0, 1fr);
		gap: 2rem;
	}

	.screen {
		border-radius: 0.875rem;
	}
}

@media (prefers-reduced-motion: reduce) {
	* {
		transition: none !important;
	}
}
</style>
