<script lang="ts">
    import "$lib/palette.css";
    import { api, type ProjectInfo } from "$lib/api";

    interface Props {
        project: ProjectInfo;
        can_ship: boolean;
        onedit: () => void;
        onchange: () => void;
    }
    let { project, can_ship, onedit, onchange }: Props = $props();

    let incomplete = $derived(
        [
            project.name,
            project.description,
            project.screenshot_url,
            project.code_url,
            project.playable_url,
        ].some((f) => !f) ||
            project.games.length === 0 ||
            project.hackatime_project_names.length === 0,
    );

    let first_time = $state(false);
    let busy = $state(false);
    let error = $state("");

    async function run(path: string, body: unknown) {
        busy = true;
        error = "";
        try {
            await api(path, body);
            onchange();
        } catch (e) {
            error = (e as Error).message;
        } finally {
            busy = false;
        }
    }

    function respond(accept: boolean) {
        if (!accept && !confirm(`Decline "${project.name}"? This deletes it.`))
            return;
        run("/dashboard/respond-invite", {
            project_id: project.project_id,
            accept,
            first_time,
        });
    }

    function ship() {
        run("/dashboard/ship", { project_id: project.project_id });
    }

    function remove() {
        if (!confirm(`Delete "${project.name}"? This can't be undone.`)) return;
        run("/dashboard/delete-project", { project_id: project.project_id });
    }

    const REVIEW_STATUS: Record<string, string> = {
        pending: "Shipped! It's in review.",
        processing: "Shipped! It's being processed.",
        approved: "Approved!",
        error: "There was a problem with your submission. A reviewer will take a look.",
    };
</script>

<div class="project-entry">
    <p class="project-entry-name">{project.name}</p>
    <p class="project-friend-name">
        {project.friend
            ? `With ${project.friend.name}`
            : "Waiting for a partner"}
    </p>

    {#if project.friend && project.me.confirmed && project.friend.confirmed}
        {#each [{ label: "You", m: project.me }, { label: project.friend.name, m: project.friend }] as { label, m } (label)}
            <p class="hours">
                {label}:
                {#if m.hackatime_linked}
                    {m.hours_logged} / {m.hours_needed}h
                {:else}
                    Hackatime not linked
                {/if}
            </p>
        {/each}
    {/if}

    {#if project.me.confirmed && !project.shipped && incomplete}
        <p class="reminder">
            Some information missing, make sure you edit before shipping!
        </p>
    {/if}

    <div class="actions">
        {#if !project.me.confirmed}
            <label class="check">
                <input type="checkbox" bind:checked={first_time} />
                My first time coding / doing a YSWS
            </label>
            <button
                class="primary"
                disabled={busy}
                onclick={() => respond(true)}>Accept</button
            >
            <button disabled={busy} onclick={() => respond(false)}
                >Decline</button
            >
        {:else if !project.friend}
            <button disabled={busy} onclick={onedit}>Edit</button>
            <button disabled={busy} onclick={remove}>Delete</button>
        {:else if !project.friend.confirmed}
            <p class="status">Waiting for {project.friend.name} to accept</p>
        {:else if project.shipped}
            <p class="status">
                {REVIEW_STATUS[project.review_status ?? "pending"]}
            </p>
        {:else}
            {#if !project.me.ready_to_ship}
                <button disabled={busy} onclick={onedit}>Edit</button>
                <button
                    class="primary"
                    disabled={busy || !can_ship}
                    title={can_ship ? "" : "Verify your identity to ship"}
                    onclick={ship}>Ready to ship</button
                >
            {:else if !project.friend.ready_to_ship}
                <button disabled={busy} onclick={onedit}>Edit</button>
                <p class="status">Waiting for {project.friend.name} to ship</p>
            {:else}
                <button class="primary" disabled={busy} onclick={ship}
                    >Retry submission</button
                >
            {/if}
        {/if}
    </div>

    {#if error}
        <p class="error">{error}</p>
    {/if}
</div>

<style>
    .project-entry {
        display: flex;
        flex-direction: column;
        justify-content: flex-start;
        align-items: center;
        gap: 8px;
        padding: 8px;
        border-radius: 8px;
        width: min(100%, 300px);
        min-height: 200px;
        background-color: var(--bg-secondary);
        border: 4px solid var(--border);
    }
    .project-entry p {
        margin: 0;
    }
    .project-entry-name {
        font-size: 1.5rem;
        color: var(--text-primary);
    }
    .project-friend-name {
        font-size: 1rem;
        color: var(--text-secondary);
    }
    .hours {
        color: var(--text-primary);
    }
    .actions {
        margin-top: auto;
        display: flex;
        flex-wrap: wrap;
        justify-content: center;
        align-items: center;
        gap: 8px;
    }
    .actions button {
        padding: 6px 12px;
        border: 4px solid var(--border);
        border-radius: 6px;
        background-color: var(--bg-tertiary);
        color: var(--text-secondary);
        font-family: inherit;
        font-size: 1rem;
        cursor: pointer;
        transition: background-color 0.3s ease;
    }
    .actions button.primary {
        background-color: var(--accent-primary);
    }
    .actions button:hover:not(:disabled) {
        background-color: var(--accent-secondary);
    }
    .actions button:disabled {
        opacity: 0.5;
        cursor: default;
    }
    .check {
        display: flex;
        align-items: center;
        gap: 8px;
        color: var(--text-primary);
    }
    .check input {
        accent-color: var(--accent-primary);
    }
    .status {
        color: var(--text-secondary);
    }
    .reminder {
        color: var(--accent-secondary);
        font-weight: bold;
        text-align: center;
    }
    .error {
        color: var(--accent-secondary);
        text-align: center;
    }
</style>
