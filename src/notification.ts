import { createApp } from 'vue'
import NotificationApp from './NotificationApp.vue'
import 'uno.css'
import './assets/styles/base.css'

createApp(NotificationApp, {
  preview: new URLSearchParams(window.location.search).get('preview') === 'true',
}).mount('#notification')
