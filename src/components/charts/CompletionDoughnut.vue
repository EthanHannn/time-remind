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

const { t, locale } = useI18n()

const tokens = ref({
  cardSurface: '#ffffff',
  surface: '#ffffff',
  textPrimary: '#161426',
  textSecondary: '#64617e',
  border: 'rgba(15, 23, 42, 0.09)',
  drink: '#357fc4',
  rest: '#28966e',
  eyeCare: '#de7c32',
  custom: '#8367b5',
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
    cardSurface: readVar(styles, '--card-surface', dark ? '#171924' : '#ffffff'),
    surface: readVar(styles, '--bg-primary', dark ? '#171924' : '#ffffff'),
    textPrimary: readVar(styles, '--text-primary', dark ? '#f4f4fa' : '#161426'),
    textSecondary: readVar(styles, '--text-secondary', dark ? '#9899b3' : '#64617e'),
    border: readVar(styles, '--border-color', dark ? 'rgba(255,255,255,0.10)' : 'rgba(15,23,42,0.09)'),
    drink: readVar(styles, '--type-drink', dark ? '#69ade8' : '#357fc4'),
    rest: readVar(styles, '--type-rest', dark ? '#4ecb9b' : '#28966e'),
    eyeCare: readVar(styles, '--type-eye-care', dark ? '#f3a661' : '#de7c32'),
    custom: readVar(styles, '--type-custom', dark ? '#b79ce6' : '#8367b5'),
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

const totalCompleted = computed(() =>
  props.dailyStats.reduce((sum, s) => sum + s.completed_count, 0),
)

const totalCount = computed(() =>
  props.dailyStats.reduce((sum, s) => sum + s.total_count, 0),
)

const overallRate = computed(() =>
  totalCount.value > 0 ? Math.round((totalCompleted.value / totalCount.value) * 100) : 0,
)

function colorForType(type: string): string {
  if (type === 'drink')
    return tokens.value.drink
  if (type === 'rest')
    return tokens.value.rest
  if (type === 'eye_care')
    return tokens.value.eyeCare
  return tokens.value.custom
}

const chartData = computed<ChartData<'doughnut'>>(() => {
  const labels = props.dailyStats.map(s => s.name)
  const data = props.dailyStats.map(s => s.completed_count)
  const colors = props.dailyStats.map(s => colorForType(s.reminder_type))

  return {
    labels,
    datasets: [{
      data,
      backgroundColor: colors,
      borderWidth: 2,
      borderColor: tokens.value.cardSurface,
      hoverOffset: 4,
    }],
  }
})

const chartOptions = computed<ChartOptions<'doughnut'>>(() => ({
  locale: locale.value,
  responsive: true,
  maintainAspectRatio: false,
  cutout: '68%',
  plugins: {
    legend: {
      rtl: locale.value === 'ar',
      textDirection: locale.value === 'ar' ? 'rtl' : 'ltr',
      display: true,
      position: 'bottom' as const,
      labels: {
        color: tokens.value.textSecondary,
        font: { size: 12 },
        boxWidth: 12,
        boxHeight: 12,
        padding: 12,
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
    ctx.direction = locale.value === 'ar' ? 'rtl' : 'ltr'
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'

    ctx.font = 'bold 24px -apple-system, sans-serif'
    ctx.fillStyle = tokens.value.textPrimary
    ctx.fillText(new Intl.NumberFormat(locale.value, { style: 'percent' }).format(overallRate.value / 100), centerX, centerY - 8)

    ctx.font = '12px -apple-system, sans-serif'
    ctx.fillStyle = tokens.value.textSecondary
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
