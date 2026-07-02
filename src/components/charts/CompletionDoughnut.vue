<script setup lang="ts">
import type { ChartData, ChartOptions, Plugin } from 'chart.js'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { Doughnut } from 'vue-chartjs'
import { useI18n } from '../../i18n'

interface ReminderStat {
  reminder_id: string
  name: string
  icon: string
  reminder_type: string
  completed_count: number
  total_count: number
}

const props = defineProps<{
  dailyStats: ReminderStat[]
}>()

const { t } = useI18n()

const TYPE_COLORS: Record<string, string> = {
  drink: '#0ea5e9',
  rest: '#10b981',
  eye_care: '#f59e0b',
}

const isDark = ref(false)
const themeObserver = ref<MutationObserver | null>(null)

function detectTheme() {
  isDark.value = document.documentElement.getAttribute('data-theme') === 'dark'
}

onMounted(() => {
  detectTheme()
  themeObserver.value = new MutationObserver(detectTheme)
  themeObserver.value.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme'],
  })
})

onUnmounted(() => {
  themeObserver.value?.disconnect()
})

const totalCompleted = computed(() =>
  props.dailyStats.reduce((sum, s) => sum + s.completed_count, 0),
)

const totalCount = computed(() =>
  props.dailyStats.reduce((sum, s) => sum + s.total_count, 0),
)

const overallRate = computed(() =>
  totalCount.value > 0 ? Math.round((totalCompleted.value / totalCount.value) * 100) : 0,
)

const chartData = computed<ChartData<'doughnut'>>(() => {
  const labels = props.dailyStats.map(s => s.name)
  const data = props.dailyStats.map(s => s.completed_count)
  const colors = props.dailyStats.map(s => TYPE_COLORS[s.reminder_type] || '#a1a1aa')

  return {
    labels,
    datasets: [{
      data,
      backgroundColor: colors,
      borderWidth: 2,
      borderColor: isDark.value ? '#18181b' : '#ffffff',
      hoverOffset: 4,
    }],
  }
})

const chartOptions = computed<ChartOptions<'doughnut'>>(() => ({
  responsive: true,
  maintainAspectRatio: false,
  cutout: '68%',
  plugins: {
    legend: {
      display: true,
      position: 'bottom' as const,
      labels: {
        color: isDark.value ? '#a1a1aa' : '#71717a',
        font: { size: 12 },
        boxWidth: 12,
        boxHeight: 12,
        padding: 12,
        usePointStyle: true,
      },
    },
    tooltip: {
      backgroundColor: isDark.value ? 'rgba(24,24,27,0.95)' : 'rgba(255,255,255,0.98)',
      titleColor: isDark.value ? '#f4f4f5' : '#18181b',
      bodyColor: isDark.value ? '#d4d4d8' : '#3f3f46',
      borderColor: isDark.value ? 'rgba(255,255,255,0.08)' : 'rgba(0,0,0,0.08)',
      borderWidth: 1,
      padding: 10,
      cornerRadius: 8,
    },
  },
}))

// 中心文字插件
const centerTextPlugin: Plugin<'doughnut'> = {
  id: 'centerText',
  afterDraw(chart) {
    const { ctx, chartArea } = chart
    const centerX = (chartArea.left + chartArea.right) / 2
    const centerY = (chartArea.top + chartArea.bottom) / 2

    ctx.save()
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'

    ctx.font = 'bold 24px -apple-system, sans-serif'
    ctx.fillStyle = isDark.value ? '#f4f4f5' : '#18181b'
    ctx.fillText(`${overallRate.value}%`, centerX, centerY - 8)

    ctx.font = '12px -apple-system, sans-serif'
    ctx.fillStyle = isDark.value ? '#a1a1aa' : '#71717a'
    ctx.fillText(t('stats.completionRateShort'), centerX, centerY + 14)
    ctx.restore()
  },
}
</script>

<template>
  <div class="doughnut-wrapper">
    <Doughnut
      :data="chartData"
      :options="chartOptions"
      :plugins="[centerTextPlugin]"
    />
  </div>
</template>

<style scoped>
.doughnut-wrapper {
  position: relative;
  height: 200px;
  width: 100%;
  max-width: 240px;
  margin: 0 auto;
}
</style>
