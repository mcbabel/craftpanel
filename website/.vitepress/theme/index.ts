import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'

import HomeLanding from './components/HomeLanding.vue'
import InstallCommand from './components/InstallCommand.vue'
import './custom.css'

export default {
	extends: DefaultTheme,
	enhanceApp({ app }) {
		app.component('HomeLanding', HomeLanding)
		app.component('InstallCommand', InstallCommand)
	},
} satisfies Theme
