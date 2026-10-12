<script lang="ts">
    import "$lib/palette.css";
    import { api, type HackatimeStatus, type ProjectInfo } from "$lib/api";

    interface Props {
        project: ProjectInfo;
        can_ship: boolean;
        onedit: () => void;
        onchange: () => void;
    }
    let { project, can_ship, onedit, onchange }: Props = $props();

    let missing_details = $derived(
        [
            project.name,
            project.description,
            project.screenshot_url,
            project.code_url,
            project.playable_url,
        ].some((f) => !f) || project.games.length === 0,
    );
    let no_hackatime = $derived(project.hackatime_project_names.length === 0);

    let first_time = $state(false);
    let busy = $state(false);
    let error = $state("");

    async function run(path: string, body: unknown) {
        busy = true;
        error = "";
        try {
            await api(path, body);
        } catch (e) {
            error = (e as Error).message;
        } finally {
            busy = false;
            onchange();
        }
    }

    function respond(accept: boolean) {
        if (!accept && !confirm(`Decline the invite to "${project.name}"?`))
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

    function unready() {
        run("/dashboard/unready", { project_id: project.project_id });
    }

    function respond_join(user_id: number, approve: boolean) {
        run("/dashboard/respond-join", {
            project_id: project.project_id,
            user_id,
            approve,
        });
    }

    function remove() {
        if (!confirm(`Delete "${project.name}"? This can't be undone.`)) return;
        run("/dashboard/delete-project", { project_id: project.project_id });
    }

    const HACKATIME_PROBLEM: Record<HackatimeStatus, string> = {
        ok: "",
        unlinked: "Hackatime not linked",
        expired: "Hackatime needs relinking",
        unreachable: "Couldn't reach Hackatime",
    };

    let ship_blocker = $derived(
        !can_ship
            ? "Verify your identity to ship"
            : missing_details
              ? "Fill in the name, description, code URL, playable URL, screenshot and games before shipping"
              : no_hackatime
                ? "Link at least one Hackatime project before shipping"
                : project.me.hours_logged === null
                  ? HACKATIME_PROBLEM[project.me.hackatime] ||
                    "Couldn't reach Hackatime"
                  : project.me.hours_logged < project.me.hours_needed
                    ? `You've logged ${project.me.hours_logged}h of your ${project.me.hours_needed}h target`
                    : "",
    );

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
        {#each [{ key: "me", label: "You", m: project.me }, { key: "friend", label: project.friend.name, m: project.friend }] as { key, label, m } (key)}
            <p class="hours">
                {label}:
                {#if m.hours_logged !== null}
                    {m.hours_logged} / {m.hours_needed}h
                {:else}
                    {HACKATIME_PROBLEM[m.hackatime] || `- / ${m.hours_needed}h`}
                {/if}
            </p>
        {/each}
    {/if}

    {#if !project.friend && project.join_requests.length}
        <div class="requests">
            {#each project.join_requests as request (request.user_id)}
                <div class="actions">
                    <p class="status">{request.name} wants to join</p>
                    <button
                        class="primary"
                        disabled={busy}
                        onclick={() => respond_join(request.user_id, true)}
                        >Approve</button
                    >
                    <button
                        disabled={busy}
                        onclick={() => respond_join(request.user_id, false)}
                        >Decline</button
                    >
                </div>
            {/each}
        </div>
    {/if}

    {#if project.me.confirmed && !project.shipped && (missing_details || no_hackatime)}
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
            <button disabled={busy} onclick={onedit}>Edit</button>
            <button disabled={busy} onclick={remove}>Delete</button>
        {:else if project.shipped}
            <p class="status">
                {REVIEW_STATUS[project.review_status ?? "pending"]}
            </p>
        {:else if project.me.submitting}
            <p class="status">Submitting for review... Refresh in a minute.</p>
        {:else if !project.me.ready_to_ship}
            <button disabled={busy} onclick={onedit}>Edit</button>
            <button
                class="primary"
                disabled={busy || !!ship_blocker}
                title={ship_blocker}
                onclick={ship}>Ready to ship</button
            >
        {:else}
            <p class="status">Waiting for {project.friend.name} to ship</p>
            <button disabled={busy} onclick={unready}>Not ready</button>
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
    .requests {
        display: flex;
        flex-direction: column;
        gap: 8px;
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
        color: var(--text-on-accent);
    }
    .actions button:hover:not(:disabled) {
        background-color: var(--accent-secondary);
        color: var(--text-on-accent);
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
        color: var(--accent-text);
        font-weight: bold;
        text-align: center;
    }
    .error {
        color: var(--accent-text);
        text-align: center;
    }
</style>
