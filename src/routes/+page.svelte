<script lang="ts">
  import { onMount } from "svelte";
  import {
    listTasks,
    toggleCompletion,
    getMonthView,
    getWeeklyStreak,
  } from "$lib/db";
  import type { TaskWithStats, MonthViewDay } from "$lib/types";
  import TaskSettingsModal from "$lib/components/TaskSettingsModal.svelte";
  import LeftArrow from "../icons/left-arrow.ico";
  import RightArrow from "../icons/right-arrow.ico";

  const GearIcon = `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"></circle><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 5 9 1.65 1.65 0 0 0 4.67 7.18l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"></path></svg>`;

  let tasks: TaskWithStats[] = [];
  let monthData: MonthViewDay[] = [];

  // Date State
  let today = new Date();
  let currentMonth = new Date(today.getFullYear(), today.getMonth(), 1);
  let selectedDate = new Date();
  let selectedDayIndex: number = Math.floor(
    selectedDate.getTime() / 1000 / 86400,
  );

  // Modals
  let showSettings = false;

  let weeklyStreak = 0;

  function getDayIndex(date: Date): number {
    const utc = Date.UTC(date.getFullYear(), date.getMonth(), date.getDate());
    return Math.floor(utc / 1000 / 86400);
  }

  async function loadTasks() {
    try {
      // List tasks to get streaks for the dashboard
      const allTasks = await listTasks();

      // Buffer arrays locally first to avoid UI flicker
      let _daily: TaskWithStats[] = [];
      let _weekly: TaskWithStats[] = [];
      let _monthly: TaskWithStats[] = [];
      let _custom: TaskWithStats[] = [];

      // Group by schedule type
      allTasks.forEach((t) => {
        if (t.schedule.type === "daily") _daily.push(t);
        else if (t.schedule.type === "weekly") _weekly.push(t);
        else if (t.schedule.type === "monthly") _monthly.push(t);
        else _custom.push(t);
      });

      // Atomic update
      tasks = allTasks; // Fix: Update the main tasks array for settings modal
      dailyTasks = _daily;
      weeklyTasks = _weekly;
      monthlyTasks = _monthly;
      customTasks = _custom;

      // Also refresh streak
      weeklyStreak = await getWeeklyStreak();
    } catch (e) {
      console.error(e);
    }
  }

  async function loadMonthView() {
    try {
      // Fetch month view
      const year = currentMonth.getFullYear();
      const month = currentMonth.getMonth() + 1;
      monthData = await getMonthView(year, month);
    } catch (e) {
      console.error(e);
    }
  }

  async function loadData() {
    await loadTasks();
    await loadMonthView();
  }

  onMount(() => {
    loadData();
  });

  async function handleToggle(task_id: number, day: number) {
    try {
      await toggleCompletion(task_id, day);
      await loadData();
    } catch (e) {
      console.error(e);
    }
  }

  // Calendar Helpers
  $: year = currentMonth.getFullYear();
  $: month = currentMonth.getMonth(); // 0-11
  $: monthName = currentMonth.toLocaleString("default", { month: "long" });
  $: todayIndex = getDayIndex(new Date());

  // Backend now returns the full grid (42 days usually, or whatever fits)
  // so we don't need daysInMonth / offset calculation for grid generation,
  // we just iterate monthData.

  function prevMonth() {
    currentMonth = new Date(year, month - 1, 1);
    loadData();
  }

  function nextMonth() {
    currentMonth = new Date(year, month + 1, 1);
    loadData();
  }

  function goToToday() {
    selectedDate = new Date();
    currentMonth = new Date(
      selectedDate.getFullYear(),
      selectedDate.getMonth(),
      1,
    );
    loadData();
  }

  // Dashboard Grouping
  $: dailyTasks = tasks.filter((t) => t.schedule.type === "daily");
  $: weeklyTasks = tasks.filter((t) => t.schedule.type === "weekly");
  $: monthlyTasks = tasks.filter((t) => t.schedule.type === "monthly");
  $: customTasks = tasks.filter((t) => t.schedule.type === "custom");
</script>

<div class="container">
  <!-- Header / Dashboard -->
  <div class="dashboard-card">
    <div class="settings-wrapper">
      <button class="settings-trigger" on:click={() => (showSettings = true)}>
        {@html GearIcon}
      </button>
    </div>

    <div class="dashboard-grid">
      <div class="task-col">
        <div class="col-header">Täglich</div>
        <div class="col-list">
          {#each dailyTasks as t}
            <div class="dash-task">
              <span class="t-title" title={t.task.title}>{t.task.title}</span>
              <span class="t-streak">{t.current_streak}</span>
            </div>
          {/each}
          {#if dailyTasks.length === 0}<span class="empty-text">-</span>{/if}
        </div>
      </div>

      <div class="task-col">
        <div class="col-header">Wöchentlich</div>
        <div class="col-list">
          {#each weeklyTasks as t}
            <div class="dash-task">
              <span class="t-title" title={t.task.title}>{t.task.title}</span>
              <span class="t-streak">{t.current_streak}</span>
            </div>
          {/each}
          {#if weeklyTasks.length === 0}<span class="empty-text">-</span>{/if}
        </div>
      </div>

      <div class="task-col">
        <div class="col-header">Monatlich</div>
        <div class="col-list">
          {#each monthlyTasks as t}
            <div class="dash-task">
              <span class="t-title" title={t.task.title}>{t.task.title}</span>
              <span class="t-streak">{t.current_streak}</span>
            </div>
          {/each}
          {#if monthlyTasks.length === 0}<span class="empty-text">-</span>{/if}
        </div>
      </div>

      <div class="task-col">
        <div class="col-header">X Tage</div>
        <div class="col-list">
          {#each customTasks as t}
            <div class="dash-task">
              <span class="t-title" title={t.task.title}>{t.task.title}</span>
              <span class="t-streak">{t.current_streak}</span>
            </div>
          {/each}
          {#if customTasks.length === 0}<span class="empty-text">-</span>{/if}
        </div>
      </div>

      <div class="streak-col">
        <div class="streak-circle">
          <span class="fire">🔥</span>
          <span class="streak-num">{weeklyStreak}</span>
        </div>
      </div>
    </div>
  </div>

  <!-- Calendar Controls -->
  <div class="cal-controls">
    <div class="left-controls">
      <div class="arrows">
        <button class="nav-btn" on:click={prevMonth}>
          <img src={LeftArrow} alt="Previous Month" width="16" height="16" />
        </button>
        <button class="nav-btn" on:click={nextMonth}>
          <img src={RightArrow} alt="Next Month" width="16" height="16" />
        </button>
      </div>
      <button class="today-btn" on:click={goToToday}>Heute</button>
    </div>
    <div class="month-label">{monthName} {year}</div>
  </div>

  <!-- Calendar Grid -->
  <div class="calendar-wrapper">
    <div class="weekdays">
      {#each ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"] as days}
        <div class="wd">{days}</div>
      {/each}
    </div>
    <div class="days-grid">
      {#each monthData as data}
        {@const d = new Date(data.day * 86400 * 1000).getDate()}
        {@const isCurrentMonth =
          new Date(data.day * 86400 * 1000).getMonth() === month}
        {@const isToday = data.day === todayIndex}

        <!-- svelte-ignore a11y-click-events-have-key-events -->
        <div class="day-cell {isCurrentMonth ? '' : 'outside'}">
          <span class="day-num {isToday ? 'is-today' : ''}">{d}</span>
          <div class="day-tasks">
            {#if data.tasks}
              {#each data.tasks as t}
                <button
                  class="task-pill {t.is_done ? 'done' : 'due'}"
                  title={t.title}
                  on:click|stopPropagation={() => handleToggle(t.id, data.day)}
                >
                  {t.title}
                </button>
              {/each}
            {/if}
          </div>
        </div>
      {/each}
    </div>
  </div>
</div>

{#if showSettings}
  <TaskSettingsModal
    {tasks}
    on:close={() => (showSettings = false)}
    on:refresh={loadData}
  />
{/if}

<style>
  .container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 20px;
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding-bottom: 20px;
  }

  /* Dashboard */
  .dashboard-card {
    background: white;
    border: 2px solid var(--border-color);
    border-radius: 20px;
    padding: 24px;
    position: relative;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  }

  .settings-wrapper {
    position: absolute;
    top: 0;
    right: 0;
    width: 60px;
    height: 60px; /* Hover area top right */
    display: flex;
    justify-content: flex-end;
    align-items: flex-start;
    padding: 16px;
    z-index: 10;
  }

  .settings-trigger {
    background: none;
    border: none;
    cursor: pointer;
    opacity: 0;
    transition: opacity 0.3s ease;
  }

  .settings-wrapper:hover .settings-trigger {
    opacity: 1;
  }
  /* Fallback if user just hovers button area, wrapper ensures corner coverage */

  .dashboard-grid {
    display: flex;
    gap: 20px;
  }

  .task-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-width: 0; /* Important for truncation in nested flex items */
  }

  .streak-col {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .col-header {
    border: 2px solid var(--border-color);
    border-radius: 8px; /* Angular */
    padding: 6px 0;
    text-align: center;
    font-weight: normal; /* Not bold */
    background: white;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1); /* Stronger shadow */
    width: 90%;
    margin: 0 auto;
  }

  .col-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .dash-task {
    display: flex;
    gap: 8px; /* Gap between title box and streak */
    justify-content: space-between;
    align-items: center;
    font-size: 0.9rem;
    /* Removed border/bg/shadow from container */
  }

  .t-title {
    flex: 1; /* Take available width */
    border: 2px solid var(--border-color);
    border-radius: 8px;
    padding: 4px 12px;
    background: white;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1); /* Stronger shadow */
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }

  .t-streak {
    border: 2px solid var(--border-color); /* Match border thickness */
    border-radius: 50%;
    width: 24px; /* Slightly larger for visibility */
    height: 24px;
    font-size: 0.75rem;
    display: flex; /* Ensure centering */
    align-items: center;
    justify-content: center;
    background: white;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1); /* Stronger shadow */
  }

  .streak-circle {
    width: 120px;
    height: 120px;
    border: 2px solid var(--border-color);
    border-radius: 50%;
    display: flex;
    flex-direction: row; /* Side by side */
    align-items: center;
    justify-content: center;
    gap: 8px; /* Space between fire and number */
    font-size: 2.5rem;
    font-weight: bold;
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
    line-height: 1; /* Reset line height for better vertical centering */
  }

  /* Ensure the fire span aligns nicely */
  .fire {
    width: 1.1em;
    display: flex;
    align-items: center;
  }

  /* Ensure number aligns */
  .streak-num {
    width: 2ch;
    display: flex;
    font-variant-numeric: tabular-nums;
    /* Removed padding-top for natural flex centering */
  }

  .empty-text {
    text-align: center;
    color: #aaa;
    font-size: 0.8rem;
  }

  /* Calendar Controls */
  .cal-controls {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    margin-bottom: 8px;
  }

  .left-controls {
    display: flex;
    gap: 12px;
    align-items: center;
  }

  .arrows {
    display: flex;
    gap: 8px;
  }

  .nav-btn {
    background: white;
    border: 2px solid var(--border-color);
    border-radius: 8px;
    font-weight: bold;
    cursor: pointer;
    width: 40px;
    height: 32px; /* Uniform height */
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 1.2rem;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    transition:
      transform 0.2s ease,
      box-shadow 0.2s ease;
  }

  .nav-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.15);
  }

  .today-btn {
    border: 2px solid var(--border-color);
    border-radius: 8px;
    padding: 0 16px;
    background: white;
    font-weight: 600;
    height: 32px; /* Uniform height */
    display: flex;
    align-items: center;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    transition:
      transform 0.2s ease,
      box-shadow 0.2s ease;
  }

  .today-btn:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.15);
  }

  .month-label {
    border: 2px solid var(--border-color);
    border-radius: 8px;
    padding: 0 16px; /* Reduced vertical padding, horizontal 16px to match today-btn */
    font-weight: 600;
    background: white;
    height: 32px; /* Uniform height, same as buttons */
    display: flex;
    align-items: center;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  /* Calendar Grid */
  .calendar-wrapper {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .weekdays {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    margin-bottom: 4px;
    justify-items: center; /* Center boxes in grid cells */
  }

  .wd {
    width: 60%; /* Smaller width */
    padding: 4px; /* Smaller padding */
    text-align: center;
    font-weight: 600;
    font-size: 0.9rem; /* Smaller font */
    background: white;
    border: 2px solid var(--border-color);
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
  }

  .days-grid {
    display: grid;
    grid-template-columns: repeat(7, 1fr);
    border: 2px solid var(--border-color);
    border-radius: 20px;
    overflow: hidden;
    background: var(--border-color); /* Grid lines color */
    gap: 2px; /* Small gap for grid lines */
    box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  }

  .day-cell {
    background: white; /* Cells white */
    padding: 8px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    min-height: 100px;
    min-width: 0; /* Important for flex child truncation */
  }

  .day-cell.outside {
    color: #999;
    /* maybe different bg? */
  }
  .day-cell.outside .day-num {
    color: #bbb;
    border-color: #eee;
  }

  .day-num {
    border: 1px solid #ccc;
    border-radius: 4px;
    padding: 0 4px;
    align-self: flex-start;
    font-size: 0.75rem;
    color: #666;
  }

  .day-num.is-today {
    background-color: #ff5c5c;
    color: white;
    border-color: #ff5c5c;
    /* Reverting to rectangle defaults (inherit padding/border-radius from .day-num) */
    /* Overriding specific properties if needed, but .day-num has radius 4px */
  }

  .day-tasks {
    display: flex;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    width: 100%; /* Ensure full width for pill truncation */
  }

  .task-pill {
    border: 1px solid var(--border-color);
    border-radius: 4px; /* Angular: reduced radius */
    padding: 4px 8px;
    font-size: 0.8rem;
    text-align: left;
    cursor: pointer;

    /* Truncation logic */
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 100%;
    display: block;
  }

  .task-pill:hover {
    opacity: 0.9;
  }

  .task-pill.done {
    background: var(--accent-green);
  }

  .task-pill.due {
    background: var(--accent-red);
  }

  /* Responsive tweaks if needed */
</style>
