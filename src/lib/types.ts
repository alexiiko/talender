export interface Task {
    id: number;
    title: string;
    notes: string | null;
    is_active: boolean;
    created_at: number;
    archived_at: number | null;
}

export interface TaskSchedule {
    id: number;
    task_id: number;
    effective_from: number;
    effective_to: number | null;
    type: 'daily' | 'weekly' | 'monthly' | 'custom';
    weekday_mask: number | null;
    monthday: number | null;
    interval_days: number | null;
    params_json: string | null;
}

export interface TaskWithStats {
    task: Task;
    schedule: TaskSchedule;
    current_streak: number;
    best_streak: number;
    today_status: boolean;
}

export interface MonthTask {
    id: number;
    title: string;
    is_done: boolean;
}

export interface MonthViewDay {
    day: number;
    due_count: number;
    done_count: number;
    all_done: boolean;
    tasks: MonthTask[];
}
