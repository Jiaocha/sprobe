import { createRouter, createWebHistory } from 'vue-router'
import Overview from '../pages/Overview.vue'
import NodeDetail from '../pages/NodeDetail.vue'
import Settings from '../pages/Settings.vue'

const routes = [
  {
    path: '/',
    redirect: '/overview',
  },
  {
    path: '/overview',
    name: 'Overview',
    component: Overview,
  },
  {
    path: '/nodes/:id',
    name: 'NodeDetail',
    component: NodeDetail,
    props: true,
  },
  {
    path: '/settings',
    name: 'Settings',
    component: Settings,
  },
  {
    path: '/:pathMatch(.*)*',
    redirect: '/overview',
  },
]

const router = createRouter({
  history: createWebHistory(),
  routes,
})

export default router
