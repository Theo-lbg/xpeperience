import { createRouter, createWebHistory } from 'vue-router'
import { can, session } from './lib/store'
import type { Area } from './lib/types'

declare module 'vue-router' {
  interface RouteMeta { public?: boolean; area?: Area }
}

export const router = createRouter({
  history: createWebHistory(),
  scrollBehavior: () => ({ top: 0 }),
  routes: [
    { path: '/login', component: () => import('./views/LoginView.vue'), meta: { public: true } },
    { path: '/', component: () => import('./views/DashboardView.vue') },
    { path: '/crm', component: () => import('./views/CrmView.vue'), meta: { area: 'crm' } },
    { path: '/crm/companies/:id(\\d+)', component: () => import('./views/CompanyView.vue'), meta: { area: 'crm' } },
    { path: '/crm/contacts/:id(\\d+)', component: () => import('./views/ContactView.vue'), meta: { area: 'crm' } },
    { path: '/needs', component: () => import('./views/NeedsView.vue'), meta: { area: 'needs' } },
    { path: '/needs/:id(\\d+)', component: () => import('./views/NeedView.vue'), meta: { area: 'needs' } },
    { path: '/resources', component: () => import('./views/ResourcesView.vue'), meta: { area: 'resources' } },
    { path: '/resources/:id(\\d+)', component: () => import('./views/ResourceView.vue'), meta: { area: 'resources' } },
    { path: '/projects', component: () => import('./views/ProjectsView.vue'), meta: { area: 'projects' } },
    { path: '/projects/:id(\\d+)', component: () => import('./views/ProjectView.vue'), meta: { area: 'projects' } },
    { path: '/settings', component: () => import('./views/SettingsView.vue') },
    { path: '/:rest(.*)*', component: () => import('./views/NotFoundView.vue') },
  ],
})

router.beforeEach((to) => {
  if (!session.loaded) return true
  if (!to.meta.public && !session.authenticated) return { path: '/login', query: to.fullPath === '/' ? {} : { next: to.fullPath } }
  if (to.path === '/login' && session.authenticated) return '/'
  if (to.meta.area && !can(to.meta.area)) return '/'
  return true
})
