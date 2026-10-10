<script lang="ts">
    import "$lib/palette.css";
    import "$lib/scrollbar.css";
    import ProjectEntry from "./ProjectEntry.svelte";
    import ProjectDialog from "./ProjectDialog.svelte";
    import DashboardLines from "./DashboardLines.svelte";
    import { onMount } from "svelte";
    import { api, type Invite, type Me, type ProjectInfo } from "$lib/api";

    let me = $state<Me | null>(null);
    let projects = $state<ProjectInfo[]>([]);
    let error = $state("");
    let dialog_open = $state(false);
    let dialog_mode = $state<"add" | "edit">("add");
    let editing = $state<ProjectInfo | null>(null);
    let partner_code = $state("");
    let invite = $state<(Invite & { code: string }) | null>(null);
    let join_first_time = $state(false);
    let joining = $state(false);
    let copied = $state(false);

    let linked = $derived(me?.hackatime_linked ?? true);

    async function refresh() {
        try {
            me = await api<Me>("/dashboard/me");
            projects = await api<ProjectInfo[]>("/dashboard/projects");
            error = "";
        } catch (e) {
            error = (e as Error).message;
        }
    }

    function take_join_code() {
        const match = document.cookie.match(/(?:^|;\s*)dyad_join=(\d+)/);
        if (!match) return null;
        document.cookie = "dyad_join=; path=/; max-age=0";
        return match[1];
    }

    async function check_invite() {
        if (!me?.hackatime_linked) return;
        const code = take_join_code();
        if (!code) return;
        try {
            const found = await api<Invite>(`/dashboard/invite/${code}`);
            if (found.project_name) invite = { ...found, code };
            else new_project(code);
        } catch (e) {
            error = (e as Error).message;
        }
    }

    onMount(async () => {
        await refresh();
        await check_invite();
    });

    async function join() {
        if (!invite) return;
        joining = true;
        try {
            await api("/dashboard/join", {
                friend_code: Number(invite.code),
                first_time: join_first_time,
            });
            await refresh();
        } catch (e) {
            error = (e as Error).message;
        } finally {
            invite = null;
            joining = false;
        }
    }

    async function copy_invite() {
        if (!me) return;
        await navigator.clipboard.writeText(
            `${location.origin}/join/${me.friend_code}`,
        );
        copied = true;
        setTimeout(() => (copied = false), 2000);
    }

    async function set_reminders(enabled: boolean) {
        try {
            await api("/dashboard/reminders", { enabled });
            if (me) me.reminders = enabled;
        } catch (e) {
            error = (e as Error).message;
        }
    }

    function new_project(code = "") {
        dialog_mode = "add";
        editing = null;
        partner_code = code;
        dialog_open = true;
    }

    function edit_project(project: ProjectInfo) {
        dialog_mode = "edit";
        editing = project;
        dialog_open = true;
    }

    function link_hackatime() {
        window.location.href = "/hackatime-login";
    }
</script>

<svelte:head>
    <title>Dyad Dashboard</title>
</svelte:head>

<DashboardLines />

<ProjectDialog
    bind:open={dialog_open}
    mode={dialog_mode}
    existing={editing}
    {partner_code}
    onsaved={refresh}
/>

<div class="top-bar">
    <div class="container">
        <h1>Dyad Dashboard</h1>
        {#if me}
            <p class="friend-code">
                Your friend code: <strong>{me.friend_code}</strong> ·
                <button class="link-button" onclick={copy_invite}
                    >{copied ? "Copied!" : "Copy invite link"}</button
                >
                ·
                <a href="/logout">Log out</a>
            </p>
            <label class="friend-code reminders">
                <input
                    type="checkbox"
                    checked={me.reminders}
                    onchange={(e) => set_reminders(e.currentTarget.checked)}
                />
                Slack me if I haven't worked on a project for a while
            </label>
        {/if}
    </div>
    {#if linked}
        <button
            class="new-project-button container"
            onclick={() => new_project()}><h1>＋ Add Project</h1></button
        >
    {/if}
</div>

{#if me && !me.ysws_eligible}
    <div class="container banner">
        You need to <a href="https://auth.hackclub.com">verify your identity</a> with
        Hack Club before you can ship.
    </div>
{/if}

{#if invite}
    <div class="container banner invite">
        <span>{invite.name} invited you to join "{invite.project_name}".</span>
        <label class="reminders">
            <input type="checkbox" bind:checked={join_first_time} />
            My first time coding / doing a YSWS
        </label>
        <button class="banner-button primary" disabled={joining} onclick={join}
            >Join</button
        >
        <button
            class="banner-button"
            disabled={joining}
            onclick={() => (invite = null)}>Dismiss</button
        >
    </div>
{/if}

{#if error}
    <div class="container banner">{error}</div>
{/if}

<div class="container project-container themed-scroll">
    {#if linked}
        {#each projects as project (project.project_id)}
            <ProjectEntry
                {project}
                can_ship={me?.ysws_eligible ?? false}
                onedit={() => edit_project(project)}
                onchange={refresh}
            />
        {:else}
            <p class="empty">
                No projects yet. Add one with your partner's friend code, or add
                one on your own and send them your invite link.
            </p>
        {/each}
    {:else}
        <div class="link-hackatime-button-container">
            <button class="link-hackatime-button" onclick={link_hackatime}
                >Link Hackatime to continue</button
            >
        </div>
    {/if}
</div>

<style>
    :root {
        margin-left: 10vw;
        margin-right: 10vw;
        margin-top: 5dvh;
        color: var(--text-primary);
    }
    :global(body) {
        display: flex;
        flex-direction: column;
        height: 90dvh;
    }
    .container {
        box-sizing: border-box;
        background-color: var(--bg-tertiary);
        border: 4px solid var(--border);
        border-radius: 16px;
        padding: 8px;
        width: fit-content;
    }
    .top-bar {
        width: 100%;
        display: flex;
        justify-content: space-between;
        align-items: stretch;
    }
    .new-project-button {
        color: var(--text-primary);
        transition: background-color 0.3s ease;
    }
    .new-project-button:hover {
        background-color: var(--accent-primary);
    }
    .friend-code {
        margin: 0;
        color: var(--text-secondary);
    }
    .friend-code a,
    .banner a {
        color: var(--accent-secondary);
    }
    .link-button {
        padding: 0;
        background: none;
        border: none;
        color: var(--accent-secondary);
        font: inherit;
        text-decoration: underline;
        cursor: pointer;
    }
    .reminders {
        display: flex;
        align-items: center;
        gap: 8px;
    }
    .reminders input {
        accent-color: var(--accent-primary);
    }
    .invite {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 12px;
    }
    .banner-button {
        padding: 6px 12px;
        border: 4px solid var(--border);
        border-radius: 6px;
        background-color: var(--bg-secondary);
        color: var(--text-secondary);
        font-family: inherit;
        font-size: 1rem;
        cursor: pointer;
        transition: background-color 0.3s ease;
    }
    .banner-button.primary {
        background-color: var(--accent-primary);
    }
    .banner-button:hover:not(:disabled) {
        background-color: var(--accent-secondary);
    }
    .banner-button:disabled {
        opacity: 0.5;
        cursor: default;
    }
    .banner {
        width: 100%;
        margin-top: 16px;
        color: var(--text-secondary);
    }
    .empty {
        color: var(--text-secondary);
    }
    .project-container {
        display: flex;
        flex-wrap: wrap;
        justify-content: center;
        align-content: flex-start;
        gap: 16px;
        flex: 1;
        min-height: 300px;
        width: 100%;
        margin-top: 5vh;
        overflow-y: auto;
        scrollbar-gutter: stable both-edges;
    }
    .link-hackatime-button {
        display: absolute;
        padding: 8px;
        color: var(--text-primary);
        background-color: var(--accent-primary);
        margin-top: 10vh;
        font-size: 36px;
        border: 4px solid var(--border);
        border-radius: 6px;
    }
    .link-hackatime-button:hover {
        background-color: var(--accent-secondary);
    }
    .link-hackatime-button-container {
        width: 100%;
        display: flex;
        justify-content: center;
    }
</style>
