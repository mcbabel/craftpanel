import { defineConfig } from 'vitepress'

const repository = 'https://github.com/MCbabel/craftpanel'

export default defineConfig({
	title: 'CraftPanel',
	description:
		'A self-hosted panel for running Minecraft servers on your own Linux machine. One install command, no containers.',
	lang: 'en-US',
	base: '/craftpanel/',
	cleanUrls: true,
	lastUpdated: true,
	head: [
		['link', { rel: 'icon', type: 'image/svg+xml', href: '/craftpanel/favicon.svg' }],
		['meta', { name: 'theme-color', content: '#16a34a' }],
		['meta', { property: 'og:title', content: 'CraftPanel' }],
		[
			'meta',
			{
				property: 'og:description',
				content: 'A self-hosted panel for running Minecraft servers on your own Linux machine.',
			},
		],
	],
	themeConfig: {
		logo: '/logo.svg',
		nav: [
			{ text: 'Install', link: '/docs/installation' },
			{ text: 'Docs', link: '/docs/' },
			{ text: 'Releases', link: `${repository}/releases` },
		],
		sidebar: {
			'/docs/': [
				{
					text: 'Getting started',
					items: [
						{ text: 'Overview', link: '/docs/' },
						{ text: 'Installation', link: '/docs/installation' },
						{ text: 'Configuration', link: '/docs/configuration' },
						{ text: 'Upgrading', link: '/docs/upgrading' },
					],
				},
				{
					text: 'Using the panel',
					items: [
						{ text: 'Accounts and access', link: '/docs/accounts' },
						{ text: 'Java runtimes', link: '/docs/java' },
					],
				},
				{
					text: 'Optional services',
					items: [
						{ text: 'Google Drive backups', link: '/docs/google-drive' },
						{ text: 'Mail', link: '/docs/mail' },
						{ text: 'Public addresses', link: '/docs/playit' },
					],
				},
				{
					text: 'Internals',
					items: [{ text: 'Architecture', link: '/docs/architecture' }],
				},
			],
		},
		socialLinks: [{ icon: 'github', link: repository }],
		search: { provider: 'local' },
		editLink: {
			// Serialised into the client bundle, so it cannot close over anything in this file.
			pattern: ({ filePath }) =>
				`https://github.com/MCbabel/craftpanel/edit/main/${filePath.replace(/^docs\/index\.md$/, 'docs/README.md')}`,
			text: 'Edit this page on GitHub',
		},
		outline: { level: [2, 3] },
		footer: {
			message: `Released under the <a href="${repository}/blob/main/LICENSE">AGPL-3.0</a>.`,
			copyright:
				'Not affiliated with Mojang Studios, Microsoft or Rinth, Inc. Minecraft is a trademark of Mojang Studios.',
		},
	},
})
