import { createApp } from 'vue'
import App from './App.vue'
import { setUnauthorizedHandler } from './lib/api'
import { boot, loadSession, session } from './lib/store'
import { router } from './router'
import './styles.css'

setUnauthorizedHandler(() => {
  boot.value = null
  session.authenticated = false
  if (router.currentRoute.value.path !== '/login') router.push('/login')
})

// L'application rouvre toujours sur le tableau de bord : la session est restaurée avant le premier affichage.
loadSession()
  .catch(() => {
    session.loaded = true
  })
  .finally(() => {
    createApp(App).use(router).mount('#app')
  })
