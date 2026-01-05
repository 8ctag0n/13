<script setup>
import { ref, onMounted, onUnmounted } from 'vue'
import { withBase } from 'vitepress'

const frames = ref([])
const currentFrame = ref(0)
const isPlaying = ref(false)
const fps = 15
let intervalId = null

const loadFrames = async () => {
  try {
    const response = await fetch(withBase('/zyb_pulse.json'))
    frames.value = await response.json()
    startAnimation()
  } catch (e) {
    console.error("Failed to load ZYB Pulse:", e)
  }
}

const startAnimation = () => {
  if (intervalId) clearInterval(intervalId)
  isPlaying.value = true
  intervalId = setInterval(() => {
    currentFrame.value = (currentFrame.value + 1) % frames.value.length
  }, 1000 / fps)
}

const togglePlay = () => {
  if (isPlaying.value) {
    clearInterval(intervalId)
    isPlaying.value = false
  } else {
    startAnimation()
  }
}

onMounted(() => {
  loadFrames()
})

onUnmounted(() => {
  if (intervalId) clearInterval(intervalId)
})
</script>

<template>
  <div class="terminal-cinema">
    <div class="terminal-header">
      <span class="dot red"></span>
      <span class="dot yellow"></span>
      <span class="dot green"></span>
      <span class="title">zyb-node --active</span>
    </div>
    <div class="screen" @click="togglePlay">
      <pre v-if="frames.length > 0">{{ frames[currentFrame].join('\n') }}</pre>
      <div v-else class="loading">Initializing ZYB Protocol...</div>
    </div>
    <div class="status-bar">
      <span>STATUS: ONLINE</span>
      <span>FHE_ENGINE: TFHE-rs</span>
      <span>CONSENSUS: 2/3</span>
    </div>
  </div>
</template>

<style scoped>
.terminal-cinema {
  background: #0d1117;
  border-radius: 8px;
  border: 1px solid #30363d;
  box-shadow: 0 0 20px rgba(0, 255, 159, 0.1);
  font-family: 'Courier New', Courier, monospace;
  margin: 2rem 0;
  overflow: hidden;
  max-width: 720px;
  margin-left: auto;
  margin-right: auto;
}

.terminal-header {
  background: #161b22;
  padding: 8px 12px;
  display: flex;
  align-items: center;
  border-bottom: 1px solid #30363d;
}

.dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  margin-right: 6px;
}

.red { background: #ff5f56; }
.yellow { background: #ffbd2e; }
.green { background: #27c93f; }

.title {
  color: #8b949e;
  font-size: 12px;
  margin-left: 10px;
}

.screen {
  padding: 20px;
  color: #00ff9f; /* Cyber Cyan */
  background: #0d1117;
  min-height: 300px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}

pre {
  margin: 0;
  line-height: 1.2;
  font-size: 14px;
  white-space: pre;
}

.status-bar {
  background: #161b22;
  padding: 4px 12px;
  color: #8b949e;
  font-size: 10px;
  display: flex;
  justify-content: space-between;
  border-top: 1px solid #30363d;
}

.loading {
  animation: blink 1s infinite;
}

@keyframes blink {
  50% { opacity: 0.5; }
}
</style>
