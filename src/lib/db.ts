import { invoke } from '@tauri-apps/api/core';
import type { TaskWithStats, MonthViewDay } from './types';

export async function addTask(
    title: string,
    frequencyType: string,
    weekdayMask: number | null,
    monthday: number | null,
    intervalDays: number | null
): Promise<void> {
    await invoke('add_task', {
        title,
        frequencyType,
        weekdayMask,
        monthday,
        intervalDays
    });
}

export async function listTasks(day?: number): Promise<TaskWithStats[]> {
    return await invoke('list_tasks', { day });
}

export async function deleteTask(taskId: number): Promise<void> {
    await invoke('delete_task', { taskId });
}

export async function deleteAllTasks(): Promise<void> {
    await invoke("delete_all_tasks");
}

export async function getWeeklyStreak(): Promise<number> {
    return await invoke("get_weekly_streak");
}

export async function toggleCompletion(taskId: number, day: number): Promise<void> {
    await invoke('toggle_completion', { taskId, day });
}

export async function getMonthView(year: number, month: number): Promise<MonthViewDay[]> {
    return await invoke('get_month_view', { year, month });
}

export async function editTask(
    taskId: number,
    newTitle: string,
    newFrequencyType: string,
    newWeekdayMask: number | null,
    newMonthday: number | null,
    newIntervalDays: number | null
): Promise<void> {
    await invoke('edit_task', {
        taskId,
        newTitle,
        newFrequencyType,
        newWeekdayMask,
        newMonthday,
        newIntervalDays
    });
}
