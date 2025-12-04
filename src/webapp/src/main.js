import { mount } from 'svelte'
import './app.css'
import App from './App.svelte'
import { initHashRouter } from './lib/stores/router'

// Initialize hash-based routing (enables navigation via URL hash)
initHashRouter()

const app = mount(App, {
  target: document.getElementById('app'),
})

export default app
