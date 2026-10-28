import 'overlayscrollbars/overlayscrollbars.css'
import '@/styles/global.scss'

import { installTooltipDirective } from '@modrinth/ui'
import { VueQueryPlugin } from '@tanstack/vue-query'
import { createApp } from 'vue'

import App from '@/App.vue'
import { installClipboardFallback } from '@/clipboard'
import { startBranding } from '@/composables/branding'
import { startLocale } from '@/composables/locale'
import { startTheme } from '@/composables/theme'
import { overlayScrollbarsDirective } from '@/directives/overlay-scrollbars'
import i18nPlugin from '@/i18n'
import router from '@/router'

startTheme()
startLocale()
startBranding()
installClipboardFallback()

const app = createApp(App)

app.use(VueQueryPlugin)
app.use(router)
app.use(i18nPlugin)
installTooltipDirective(app)
app.directive('overlay-scrollbars', overlayScrollbarsDirective)

app.mount('#app')
