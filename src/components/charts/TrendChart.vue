<script setup lang="ts">
import type { ChartData, ChartOptions } from 'chart.js'
import { computed, onMounted, onUnmounted, ref } from 'vue'
import { Chart as ChartComponent } from 'vue-chartjs'
import { useI18n } from '../../i18n'

interface TrendStat {
  date: string
  completed_count: number
  total_count: number
}

const props = defineProps<{
  trendStats: TrendStat[]
}>()

const { t } = useI18n()

// 主题色（与 uno.config.ts 保持一致）
const BRAND_PRIMARY = '#0ea5e9'
const BRAND_SUCCESS = '#10b981'

// 跟踪暗色主题，决定文字/网格色
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

const textColor = computed(() => isDark.value ? '#a1a1aa' : '#71717a')
const gridColor = computed(() => isDark.value ? 'rgba(255,255,255,0.06)' : 'rgba(0,0,0,0.06)')

// 日期短格式 M/D
function formatDate(dateStr: string): string {
  const date = new Date(dateStr)
  return `${date.getMonth() + 1}/${date.getDate()}`
}

const chartData = computed<ChartData<'bar' | 'line'>>(() => {
  const labels = props.trendStats.map(s => formatDate(s.date))
  const completed = props.trendStats.map(s => s.completed_count)
  const rate = props.trendStats.map(s =>
    s.total_count > 0 ? Math.round((s.completed_count / s.total_count) * 100) : 0,
  )

  return {
    labels,
    datasets: [
      {
        type: 'bar' as const,
        label: t('stats.completedCount'),
        data: completed,
        backgroundColor: `${BRAND_PRIMARY}cc`,
        borderColor: BRAND_PRIMARY,
        borderWidth: 0,
        borderRadius: 6,
        borderSkipped: false,
        yAxisID: 'y',
        order: 2,
      },
      {
        type: 'line' as const,
        label: t('stats.completionRate'),
        data: rate,
        borderColor: BRAND_SUCCESS,
        backgroundColor: `${BRAND_SUCCESS}33`,
        borderWidth: 2,
        pointRadius: 3,
        pointHoverRadius: 5,
        pointBackgroundColor: BRAND_SUCCESS,
        tension: 0.35,
        fill: true,
        yAxisID: 'y1',
        order: 1,
      },
    ],
  }
})

const chartOptions = computed<ChartOptions<'bar'>>(() => ({
  responsive: true,
  maintainAspectRatio: false,
  interaction: {
    mode: 'index' as const,
    intersect: false,
  },
  plugins: {
    legend: {
      display: true,
      position: 'top' as const,
      align: 'end' as const,
      labels: {
        color: textColor.value,
        font: { size: 12 },
        boxWidth: 12,
        boxHeight: 12,
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
      displayColors: true,
      boxPadding: 4,
    },
  },
  scales: {
    x: {
      grid: { display: false },
      ticks: { color: textColor.value, font: { size: 11 } },
      border: { display: false },
    },
    y: {
      beginAtZero: true,
      position: 'left' as const,
      grid: { color: gridColor.value },
      ticks: { color: textColor.value, font: { size: 11 }, precision: 0 },
      border: { display: false },
    },
    y1: {
      beginAtZero: true,
      max: 100,
      position: 'right' as const,
      grid: { display: false },
      ticks: {
        color: textColor.value,
        font: { size: 11 },
        callback: value => `${value}%`,
      },
      border: { display: false },
    },
  },
}))
</script>

<template>
  <div class="trend-chart-wrapper">
    <ChartComponent
      type="bar"
      :data="chartData"
      :options="chartOptions"
    />
  </div>
</template>

<style scoped>
.trend-chart-wrapper {
  position: relative;
  height: 200px;
  width: 100%;
}
</style>
