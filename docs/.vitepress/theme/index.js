import DefaultTheme from 'vitepress/theme'
import TerminalCinema from './components/TerminalCinema.vue'
import './custom.css'

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component('TerminalCinema', TerminalCinema)
  }
}
