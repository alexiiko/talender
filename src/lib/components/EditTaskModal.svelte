<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { editTask } from "../db";
    import type { TaskWithStats } from "../types";

    export let task: TaskWithStats;

    const dispatch = createEventDispatcher();

    let title = task.task.title;
    let frequency: string = task.schedule.type;
    let weekdays = new Set<number>();
    if (task.schedule.weekday_mask) {
        for (let i = 0; i < 7; i++) {
            if ((task.schedule.weekday_mask & (1 << i)) !== 0) {
                weekdays.add(i);
            }
        }
    }
    let monthday = task.schedule.monthday || 1;
    let intervalDays = task.schedule.interval_days || 1;

    // German labels for display (internal values stay English)
    const frequencyLabels: Record<string, string> = {
        daily: "Täglich",
        weekly: "Wöchentlich",
        monthly: "Monatlich",
        custom: "X Tage",
    };

    const days = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];

    function toggleWeekday(index: number) {
        if (weekdays.has(index)) {
            weekdays.delete(index);
        } else {
            weekdays.add(index);
        }
        weekdays = weekdays;
    }

    async function handleEdit() {
        if (!title.trim()) return;

        let mask = null;
        if (frequency === "weekly") {
            mask = 0;
            weekdays.forEach((d) => {
                mask! |= 1 << d;
            });
            if (mask === 0) return;
        }

        let mDay = null;
        if (frequency === "monthly") {
            mDay = monthday;
        }

        let interval = null;
        if (frequency === "custom") {
            interval = intervalDays;
        }

        try {
            await editTask(
                task.task.id,
                title,
                frequency,
                mask,
                mDay,
                interval,
            );
            dispatch("close");
            dispatch("updated");
        } catch (e) {
            console.error(e);
            alert("Failed to update task");
        }
    }
</script>

<div
    class="modal-overlay"
    on:click={() => dispatch("close")}
    on:keydown={() => {}}
>
    <div
        class="modal"
        on:click|stopPropagation={() => {}}
        on:keydown={() => {}}
    >
        <div class="modal-header">
            <button class="close-btn" on:click={() => dispatch("close")}
                >&times;</button
            >
            <span>Aufgabe bearbeiten</span>
            <button class="close-btn" style="visibility: hidden;"
                >&times;</button
            >
        </div>

        <div class="form-group">
            <label class="form-label" for="edit-task-name">Aufgabenname:</label>
            <input
                id="edit-task-name"
                type="text"
                class="input-field"
                bind:value={title}
                autocomplete="off"
            />
        </div>

        <div class="form-group">
            <span class="form-label">Häufigkeit:</span>
            <div class="frequency-options">
                {#each ["daily", "weekly", "monthly", "custom"] as type}
                    <button
                        class="chip {frequency === type ? 'selected' : ''}"
                        on:click={() => (frequency = type)}
                    >
                        {frequencyLabels[type]}
                    </button>
                {/each}
            </div>
        </div>

        {#if frequency === "weekly"}
            <div class="form-group">
                <div class="weekdays-grid">
                    {#each days as day, i}
                        <button
                            class="weekday-chip {weekdays.has(i)
                                ? 'selected'
                                : ''}"
                            on:click={() => toggleWeekday(i)}
                        >
                            {day}
                        </button>
                    {/each}
                </div>
            </div>
        {/if}

        {#if frequency === "monthly"}
            <div class="form-group">
                <label class="form-label" for="edit-month-day"
                    >Tag des Monats:</label
                >
                <select
                    id="edit-month-day"
                    class="input-field"
                    bind:value={monthday}
                >
                    {#each Array(28) as _, i}
                        <option value={i + 1}>{i + 1}</option>
                    {/each}
                </select>
            </div>
        {/if}

        {#if frequency === "custom"}
            <div class="form-group">
                <label class="form-label" for="edit-interval-days"
                    >Jede X Tage:</label
                >
                <div style="display: flex; gap: 8px; align-items: center;">
                    <select
                        id="edit-interval-days"
                        class="input-field"
                        bind:value={intervalDays}
                        style="width: 80px;"
                    >
                        {#each Array(31) as _, i}
                            <option value={i + 1}>{i + 1}</option>
                        {/each}
                    </select>
                    <span>Tage</span>
                </div>
            </div>
        {/if}

        <button
            class="btn-primary"
            style="margin-top: auto; align-self: flex-end;"
            on:click={handleEdit}
        >
            Bearbeiten
        </button>
    </div>
</div>
