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

const { t, locale } = useI18n()

const tokens = ref({
  isDark: false,
  primary: '#3b66f5',
  success: '#10a37f',
  surface: '#ffffff',
  textPrimary: '#161426',
  textSecondary: '#64617e',
  border: 'rgba(15, 23, 42, 0.09)',
})

let themeObserver: MutationObserver | null = null

function readVar(styles: CSSStyleDeclaration, name: string, fallback: string): string {
  const value = styles.getPropertyValue(name).trim()
  return value || fallback
}

function detectTheme() {
  const dark = document.documentElement.getAttribute('data-theme') === 'dark'
  const styles = getComputedStyle(document.documentElement)
  tokens.value = {
    isDark: dark,
    primary: readVar(styles, '--color-primary', dark ? '#8b85ff' : '#3b66f5'),
    success: readVar(styles, '--type-rest', dark ? '#3ad4a8' : '#10a37f'),
    surface: readVar(styles, '--bg-primary', dark ? '#171924' : '#ffffff'),
    textPrimary: readVar(styles, '--text-primary', dark ? '#f4f4fa' : '#161426'),
    textSecondary: readVar(styles, '--text-secondary', dark ? '#9899b3' : '#64617e'),
    border: readVar(styles, '--border-color', dark ? 'rgba(255,255,255,0.10)' : 'rgba(15,23,42,0.09)'),
  }
}

onMounted(() => {
  detectTheme()
  themeObserver = new MutationObserver(detectTheme)
  themeObserver.observe(document.documentElement, {
    attributes: true,
    attributeFilter: ['data-theme', 'data-palette'],
  })
})

onUnmounted(() => {
  themeObserver?.disconnect()
})

const gridColor = computed(() => (tokens.value.isDark ? 'rgba(255,255,255,0.07)' : 'rgba(15,23,42,0.06)'))

// Format calendar dates in the selected locale without shifting the day.
function formatDate(dateStr: string): string {
  const date = new Date(dateStr)
  return new Intl.DateTimeFormat(locale.value, { month: 'numeric', day: 'numeric', timeZone: 'UTC' }).format(date)
}

function withAlpha(color: string, hexAlpha: string): string {
  return /^#[\da-f]{6}$/i.test(color) ? `${color}${hexAlpha}` : color
}

const chartData = computed<ChartData<'bar' | 'line'>>(() => {
  const labels = props.trendStats.map(s => formatDate(s.date))
  const completed = props.trendStats.map(s => s.completed_count)
  const rate = props.trendStats.map(s =>
    s.total_count > 0 ? Math.round((s.completed_count / s.total_count) * 100) : 0,
  )
  const primary = tokens.value.primary
  const success = tokens.value.success

  return {
    labels,
    datasets: [
      {
        type: 'bar' as const,
        label: t('stats.completedCount'),
        data: completed,
        backgroundColor: withAlpha(primary, 'cc'),
        borderColor: primary,
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
        borderColor: success,
        backgroundColor: withAlpha(success, '26'),
        borderWidth: 2,
        pointRadius: 3,
        pointHoverRadius: 5,
        pointBackgroundColor: success,
        tension: 0.35,
        fill: true,
        yAxisID: 'y1',
        order: 1,
      },
    ],
  }
})

const chartOptions = computed<ChartOptions<'bar'>>(() => ({
  locale: locale.value,
  responsive: true,
  maintainAspectRatio: false,
  interaction: {
    mode: 'index' as const,
    intersect: false,
  },
  plugins: {
    legend: {
      rtl: locale.value === 'ar',
      textDirection: locale.value === 'ar' ? 'rtl' : 'ltr',
      display: true,
      position: 'top' as const,
      align: 'end' as const,
      labels: {
        color: tokens.value.textSecondary,
        font: { size: 12 },
        boxWidth: 12,
        boxHeight: 12,
        usePointStyle: true,
      },
    },
    tooltip: {
      rtl: locale.value === 'ar',
      textDirection: locale.value === 'ar' ? 'rtl' : 'ltr',
      backgroundColor: tokens.value.surface,
      titleColor: tokens.value.textPrimary,
      bodyColor: tokens.value.textSecondary,
      borderColor: tokens.value.border,
      borderWidth: 1,
      padding: 10,
      cornerRadius: 10,
      displayColors: true,
      boxPadding: 4,
    },
  },
  scales: {
    x: {
      grid: { display: false },
      ticks: { color: tokens.value.textSecondary, font: { size: 11 } },
      border: { display: false },
    },
    y: {
      beginAtZero: true,
      position: 'left' as const,
      grid: { color: gridColor.value },
      ticks: { color: tokens.value.textSecondary, font: { size: 11 }, precision: 0 },
      border: { display: false },
    },
    y1: {
      beginAtZero: true,
      max: 100,
      position: 'right' as const,
      grid: { display: false },
      ticks: {
        color: tokens.value.textSecondary,
        font: { size: 11 },
        callback: value => new Intl.NumberFormat(locale.value, { style: 'percent' }).format(Number(value) / 100),
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
