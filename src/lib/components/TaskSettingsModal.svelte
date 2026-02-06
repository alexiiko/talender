<script lang="ts">
    import { createEventDispatcher } from "svelte";
    import { deleteTask, deleteAllTasks } from "../db";
    import type { TaskWithStats } from "../types";
    import EditTaskModal from "./EditTaskModal.svelte";
    import AddTaskModal from "./AddTaskModal.svelte";

    export let tasks: TaskWithStats[] = [];

    const dispatch = createEventDispatcher();

    let showAdd = false;
    let editingTask: TaskWithStats | null = null;

    // German labels for display
    const frequencyLabels: Record<string, string> = {
        daily: "Täglich",
        weekly: "Wöchentlich",
        monthly: "Monatlich",
        custom: "X Tage",
    };

    async function handleDelete(id: number) {
        if (!confirm("Delete this task?")) return;
        try {
            await deleteTask(id);
            dispatch("refresh");
        } catch (e) {
            console.error(e);
        }
    }

    async function handleDeleteAll() {
        if (
            !confirm(
                "Are you sure you want to delete ALL tasks? This cannot be undone.",
            )
        )
            return;

        try {
            await deleteAllTasks();
            dispatch("refresh");
        } catch (e) {
            console.error(e);
        }
    }
</script>

{#if showAdd}
    <AddTaskModal
        on:close={() => (showAdd = false)}
        on:added={() => {
            dispatch("refresh");
            showAdd = false;
        }}
    />
{:else if editingTask}
    <EditTaskModal
        task={editingTask}
        on:close={() => (editingTask = null)}
        on:updated={() => {
            dispatch("refresh");
            editingTask = null;
        }}
    />
{:else}
    <!-- svelte-ignore a11y-click-events-have-key-events -->
    <!-- svelte-ignore a11y-no-static-element-interactions -->
    <div class="modal-overlay" on:click={() => dispatch("close")}>
        <div
            class="modal"
            style="width: 800px; max-width: 90%; height: 80vh;"
            on:click|stopPropagation={() => {}}
        >
            <div class="modal-header">
                <div style="display: flex; gap: 12px; align-items: center;">
                    <button
                        class="icon-btn-large"
                        on:click={() => dispatch("close")}
                        aria-label="Close"
                    >
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="24"
                            height="24"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            ><line x1="18" y1="6" x2="6" y2="18"></line><line
                                x1="6"
                                y1="6"
                                x2="18"
                                y2="18"
                            ></line></svg
                        >
                    </button>
                    <span style="font-size: 1.2rem; font-weight: 500;"
                        >Aufgabeneinstellungen</span
                    >
                </div>
                <div style="display: flex; gap: 8px;">
                    <button class="btn-outline" on:click={handleDeleteAll}>
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="16"
                            height="16"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            ><polyline points="3 6 5 6 21 6"></polyline><path
                                d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                            ></path></svg
                        >
                        Alle Aufgaben löschen
                    </button>
                    <button
                        class="btn-outline square"
                        on:click={() => (showAdd = true)}
                        aria-label="Add task"
                    >
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="20"
                            height="20"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            stroke-linecap="round"
                            stroke-linejoin="round"
                            ><line x1="12" y1="5" x2="12" y2="19"></line><line
                                x1="5"
                                y1="12"
                                x2="19"
                                y2="12"
                            ></line></svg
                        >
                    </button>
                </div>
            </div>

            <div class="tasks-grid">
                <!-- Group by Type -->
                {#each ["daily", "weekly", "monthly", "custom"] as type}
                    {@const typeTasks = tasks.filter(
                        (t) => t.schedule.type === type,
                    )}
                    {#if typeTasks.length > 0}
                        <div class="task-category">
                            <div class="category-header">
                                {frequencyLabels[type]}
                            </div>
                            <div class="category-list">
                                {#each typeTasks as t}
                                    <div class="task-item-settings">
                                        <span
                                            class="task-title"
                                            title={t.task.title}
                                            >{t.task.title}</span
                                        >
                                        <button
                                            class="icon-btn"
                                            on:click={() => (editingTask = t)}
                                            aria-label="Edit task"
                                        >
                                            <!-- Pen Icon -->
                                            <svg
                                                xmlns="http://www.w3.org/2000/svg"
                                                width="16"
                                                height="16"
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="2"
                                                stroke-linecap="round"
                                                stroke-linejoin="round"
                                                ><path
                                                    d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"
                                                ></path></svg
                                            >
                                        </button>
                                        <button
                                            class="icon-btn"
                                            style="color: var(--accent-red);"
                                            on:click={() =>
                                                handleDelete(t.task.id)}
                                            aria-label="Delete task"
                                        >
                                            <!-- Trash Icon -->
                                            <svg
                                                xmlns="http://www.w3.org/2000/svg"
                                                width="16"
                                                height="16"
                                                viewBox="0 0 24 24"
                                                fill="none"
                                                stroke="currentColor"
                                                stroke-width="2"
                                                stroke-linecap="round"
                                                stroke-linejoin="round"
                                                ><polyline points="3 6 5 6 21 6"
                                                ></polyline><path
                                                    d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                                                ></path><line
                                                    x1="10"
                                                    y1="11"
                                                    x2="10"
                                                    y2="17"
                                                ></line><line
                                                    x1="14"
                                                    y1="11"
                                                    x2="14"
                                                    y2="17"
                                                ></line></svg
                                            >
                                        </button>
                                    </div>
                                {/each}
                            </div>
                        </div>
                    {/if}
                {/each}

                {#if tasks.length === 0}
                    <div style="text-align: center; color: #888;">
                        Keine Aufgaben vorhanden
                    </div>
                {/if}
            </div>
        </div>
    </div>
{/if}

<style>
    .tasks-grid {
        display: grid;
        grid-template-columns: repeat(4, 1fr);
        gap: 24px;
        overflow-y: auto;
        padding-bottom: 20px;
    }
    .task-category {
        min-width: 0;
        overflow: hidden; /* Prevent category from overflowing */
    }
    .category-header {
        border: 2px solid black;
        border-radius: 8px; /* Slightly less rounded than 20px to match image typically */
        padding: 8px 16px;
        text-align: center;
        margin-bottom: 12px;
        font-weight: 500;
        background: white;
    }
    .category-list {
        display: flex;
        flex-direction: column;
        gap: 8px;
        align-items: center; /* Centered items */
    }
    .task-item-settings {
        display: inline-flex; /* Fit content */
        gap: 8px;
        justify-content: space-between;
        align-items: center;
        border: 2px solid black;
        border-radius: 8px;
        padding: 4px 8px; /* Compact padding */
        background: white;
        max-width: 90%; /* Constrain to column width */
    }
    .task-title {
        white-space: nowrap;
        overflow: hidden;
        text-overflow: ellipsis;
        max-width: 70px; /* Reduced for tighter fit */
        display: block;
    }
    .icon-btn {
        background: none;
        border: none;
        padding: 2px;
        cursor: pointer;
        opacity: 0.6;
        transition: opacity 0.2s;
        display: flex;
    }
    .icon-btn:hover {
        opacity: 1;
    }
    .icon-btn-large {
        background: none;
        border: none;
        cursor: pointer;
        padding: 0;
        display: flex;
        align-items: center;
    }
    .btn-outline {
        background: white;
        border: 2px solid black;
        border-radius: 8px;
        padding: 6px 12px;
        font-weight: 500;
        cursor: pointer;
        display: flex;
        gap: 6px;
        align-items: center;
        transition: transform 0.1s;
    }
    .btn-outline:active {
        transform: scale(0.98);
    }
    .btn-outline.square {
        padding: 6px;
    }
</style>
