<script setup lang="ts">
import { invoke } from '@tauri-apps/api/core'
import { computed, onMounted, ref } from 'vue'
import { useI18n } from '../i18n'
import { emptyReminders, getLocalizedReminderVisual } from '../utils/reminderVisuals'
import CompletionDoughnut from './charts/CompletionDoughnut.vue'
import TrendChart from './charts/TrendChart.vue'

const emit = defineEmits<{
  close: []
}>()

interface ReminderStat {
  reminder_id: string
  name: string
  icon: string
  reminder_type: string
  completed_count: number
  total_count: number
}

interface TrendStat {
  date: string
  completed_count: number
  total_count: number
}

interface StreakInfo {
  reminder_id: string
  name: string
  icon: string
  reminder_type: string
  current_streak: number
  max_streak: number
}

const dailyStats = ref<ReminderStat[]>([])
const trendStats = ref<TrendStat[]>([])
const streakStats = ref<StreakInfo[]>([])
const selectedDays = ref(7)
const { locale, t } = useI18n()

const hasOverviewData = computed(() => {
  return dailyStats.value.some(stat => stat.total_count > 0)
    || streakStats.value.some(streak => streak.current_streak > 0 || streak.max_streak > 0)
    || trendStats.value.some(stat => stat.total_count > 0)
})

onMounted(async () => {
  await loadStats()
})

async function loadStats() {
  try {
    dailyStats.value = await invoke('get_daily_stats')
    trendStats.value = await invoke('get_trend_stats', { days: selectedDays.value })
    streakStats.value = await invoke('get_streak_stats')
  }
  catch (err) {
    console.error('Failed to load stats:', err)
  }
}

async function changeDays(days: number) {
  selectedDays.value = days
  try {
    trendStats.value = await invoke('get_trend_stats', { days })
  }
  catch (err) {
    console.error('Failed to load trend stats:', err)
  }
}

function getCompletionRate(completed: number, total: number): number {
  if (total === 0)
    return 0

  return Math.round((completed / total) * 100)
}

function getVisual(type: string) {
  return getLocalizedReminderVisual(type, locale.value)
}
</script>

<template>
  <div class="stats-overlay" @click.self="emit('close')">
    <div class="stats-container">
      <div class="stats-header">
        <div>
          <h2 class="stats-title">
            {{ t('stats.title') }}
          </h2>
          <p class="stats-subtitle">
            {{ t('stats.subtitle') }}
          </p>
        </div>

        <button class="close-button" type="button" @click="emit('close')">
          <span class="close-symbol">×</span>
        </button>
      </div>

      <div class="stats-content">
        <div v-if="!hasOverviewData" class="stats-empty">
          <img :src="emptyReminders" :alt="t('stats.emptyAlt')" class="stats-empty-image">
          <h3 class="stats-empty-title">
            {{ t('stats.emptyTitle') }}
          </h3>
          <p class="stats-empty-description">
            {{ t('stats.emptyDescription') }}
          </p>
        </div>

        <template v-else>
          <section class="stats-section">
            <div class="section-heading">
              <h3 class="section-title">
                {{ t('stats.today') }}
              </h3>
            </div>

            <div class="daily-overview">
              <div class="daily-stats">
                <article
                  v-for="stat in dailyStats"
                  :key="stat.reminder_id"
                  class="stat-card"
                >
                  <div class="stat-visual" :style="{ '--stat-accent': getVisual(stat.reminder_type).accentSoft }">
                    <img
                      v-if="getVisual(stat.reminder_type).iconAsset"
                      :src="getVisual(stat.reminder_type).iconAsset"
                      :alt="stat.name"
                      class="stat-image"
                    >
                  </div>

                  <div class="stat-info">
                    <div class="stat-row">
                      <span class="stat-name">{{ stat.name }}</span>
                      <span class="stat-rate">{{ getCompletionRate(stat.completed_count, stat.total_count) }}%</span>
                    </div>

                    <div class="progress-bar">
                      <div
                        class="progress-fill"
                        :style="{
                          width: `${getCompletionRate(stat.completed_count, stat.total_count)}%`,
                          background: getVisual(stat.reminder_type).accent,
                        }"
                      />
                    </div>

                    <div class="stat-foot">
                      <span>{{ stat.completed_count }}/{{ stat.total_count }}</span>
                      <span>{{ getVisual(stat.reminder_type).description }}</span>
                    </div>
                  </div>
                </article>
              </div>

              <div v-if="dailyStats.length > 0" class="daily-doughnut">
                <CompletionDoughnut :daily-stats="dailyStats" />
              </div>
            </div>
          </section>

          <section class="stats-section">
            <div class="section-heading">
              <h3 class="section-title">
                {{ t('stats.streak') }}
              </h3>
            </div>

            <div class="streak-grid">
              <article
                v-for="streak in streakStats"
                :key="streak.reminder_id"
                class="streak-card"
              >
                <div class="streak-visual" :style="{ '--streak-accent': getVisual(streak.reminder_type).accentSoft }">
                  <img
                    v-if="getVisual(streak.reminder_type).iconAsset"
                    :src="getVisual(streak.reminder_type).iconAsset"
                    :alt="streak.name"
                    class="streak-image"
                  >
                </div>

                <div class="streak-copy">
                  <span class="streak-name">{{ streak.name }}</span>
                  <div class="streak-metrics">
                    <div class="streak-metric">
                      <span class="metric-value">{{ streak.current_streak }}</span>
                      <span class="metric-label">{{ t('stats.currentStreak') }}</span>
                    </div>
                    <div class="streak-metric">
                      <span class="metric-value">{{ streak.max_streak }}</span>
                      <span class="metric-label">{{ t('stats.maxStreak') }}</span>
                    </div>
                  </div>
                </div>
              </article>
            </div>
          </section>

          <section class="stats-section">
            <div class="section-heading section-heading-split">
              <h3 class="section-title">
                {{ t('stats.trend') }}
              </h3>

              <div class="trend-selector">
                <button
                  class="trend-button"
                  :class="{ 'trend-button-active': selectedDays === 7 }"
                  type="button"
                  @click="changeDays(7)"
                >
                  {{ t('stats.sevenDays') }}
                </button>
                <button
                  class="trend-button"
                  :class="{ 'trend-button-active': selectedDays === 30 }"
                  type="button"
                  @click="changeDays(30)"
                >
                  {{ t('stats.thirtyDays') }}
                </button>
              </div>
            </div>

            <div class="trend-panel">
              <TrendChart v-if="trendStats.length > 0" :trend-stats="trendStats" />
              <div v-else class="empty-inline">
                {{ t('stats.emptyTrend') }}
              </div>
            </div>
          </section>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.stats-overlay {
  position: fixed;
  inset: 0;
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 20px;
  background: rgba(7, 10, 16, 0.48);
  backdrop-filter: blur(8px);
}

.stats-container {
  width: min(100%, 760px);
  max-height: 88vh;
  overflow: hidden;
  border-radius: 28px;
  border: 1px solid rgba(148, 163, 184, 0.16);
  background:
    radial-gradient(circle at top right, rgba(47, 159, 216, 0.12), transparent 28%),
    linear-gradient(180deg, rgba(255, 255, 255, 0.96), rgba(255, 255, 255, 0.92));
  box-shadow: 0 30px 60px rgba(15, 23, 42, 0.22);
  display: flex;
  flex-direction: column;
}

[data-theme='dark'] .stats-container {
  background:
    radial-gradient(circle at top right, rgba(47, 159, 216, 0.16), transparent 28%),
    linear-gradient(180deg, rgba(24, 28, 37, 0.96), rgba(16, 20, 28, 0.94));
}

.stats-header {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 24px 24px 18px;
  border-bottom: 1px solid rgba(148, 163, 184, 0.14);
}

.stats-title {
  margin: 0;
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
}

.stats-subtitle {
  margin: 6px 0 0;
  font-size: 13px;
  color: var(--text-secondary);
}

.close-button {
  width: 36px;
  height: 36px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 12px;
  background: rgba(148, 163, 184, 0.12);
  transition: transform 0.2s ease, background-color 0.2s ease;
}

.close-button:hover {
  background: rgba(148, 163, 184, 0.18);
}

.close-button:active {
  transform: scale(0.94);
}

.close-symbol {
  font-size: 20px;
  line-height: 1;
}

.stats-content {
  padding: 0 24px 24px;
  overflow-y: auto;
}

.stats-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  padding: 36px 20px 28px;
}

.stats-empty-image {
  width: min(220px, 68%);
  margin-bottom: 16px;
}

.stats-empty-title {
  margin: 0;
  font-size: 22px;
  color: var(--text-primary);
}

.stats-empty-description {
  margin: 8px 0 0;
  max-width: 420px;
  color: var(--text-secondary);
  font-size: 14px;
  line-height: 1.7;
}

.stats-section {
  padding-top: 22px;
}

.section-heading {
  margin-bottom: 16px;
}

.section-heading-split {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.section-title {
  margin: 0;
  font-size: 16px;
  font-weight: 700;
  color: var(--text-secondary);
}

.daily-overview {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 18px;
}

@media (min-width: 680px) {
  .daily-overview {
    grid-template-columns: minmax(0, 1fr) 240px;
    align-items: center;
  }
}

.daily-stats,
.streak-grid {
  display: grid;
  gap: 12px;
}

.stat-card,
.streak-card {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px;
  border-radius: 20px;
  border: 1px solid rgba(148, 163, 184, 0.14);
  background: rgba(255, 255, 255, 0.66);
}

[data-theme='dark'] .stat-card,
[data-theme='dark'] .streak-card {
  background: rgba(24, 29, 38, 0.84);
}

.stat-visual,
.streak-visual {
  width: 52px;
  height: 52px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 16px;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.56), var(--stat-accent, var(--streak-accent)));
}

.stat-image,
.streak-image {
  width: 34px;
  height: 34px;
  object-fit: contain;
}

.stat-info,
.streak-copy {
  min-width: 0;
  flex: 1;
}

.stat-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.stat-name,
.streak-name {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-primary);
}

.stat-rate {
  font-size: 12px;
  font-weight: 700;
  color: var(--text-secondary);
}

.progress-bar {
  height: 8px;
  margin-top: 10px;
  overflow: hidden;
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.18);
}

.progress-fill {
  height: 100%;
  border-radius: inherit;
  transition: width 0.3s ease;
}

.stat-foot {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  margin-top: 8px;
  font-size: 12px;
  color: var(--text-secondary);
}

.daily-doughnut {
  width: 100%;
  max-width: 240px;
  justify-self: center;
}

.streak-metrics {
  display: flex;
  gap: 18px;
  margin-top: 8px;
}

.streak-metric {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.metric-value {
  font-size: 22px;
  font-weight: 700;
  color: var(--text-primary);
  letter-spacing: -0.03em;
}

.metric-label {
  font-size: 12px;
  color: var(--text-secondary);
}

.trend-selector {
  display: flex;
  gap: 8px;
}

.trend-button {
  padding: 6px 12px;
  border-radius: 999px;
  border: 1px solid rgba(148, 163, 184, 0.16);
  background: rgba(255, 255, 255, 0.72);
  color: var(--text-primary);
  font-size: 12px;
  font-weight: 600;
}

[data-theme='dark'] .trend-button {
  background: rgba(24, 29, 38, 0.84);
}

.trend-button-active {
  background: linear-gradient(135deg, #2f9fd8, #4f8cff);
  color: white;
  border-color: transparent;
}

.trend-panel {
  min-height: 220px;
  border-radius: 20px;
  border: 1px solid rgba(148, 163, 184, 0.14);
  background: rgba(255, 255, 255, 0.66);
  padding: 10px 8px 6px;
}

[data-theme='dark'] .trend-panel {
  background: rgba(24, 29, 38, 0.84);
}

.empty-inline {
  min-height: 200px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 14px;
  color: var(--text-secondary);
}
</style>
