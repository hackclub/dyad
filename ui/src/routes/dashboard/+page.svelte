<script lang="ts">
    import "$lib/palette.css";
    import "$lib/scrollbar.css";
    import ProjectEntry from "./ProjectEntry.svelte";
    import ProjectDialog from "./ProjectDialog.svelte";
    import DashboardLines from "./DashboardLines.svelte";
    import HelpDialog from "./HelpDialog.svelte";
    import { onMount } from "svelte";
    import {
        api,
        ApiError,
        type Dashboard,
        type Invite,
        type Me,
        type PartnerInvite,
        type ProjectInfo,
    } from "$lib/api";

    const LINK_ERRORS: Record<string, string> = {
        taken: "That Hackatime account is already linked to another Dyad account.",
        failed: "Couldn't link Hackatime. Try again.",
    };

    let me = $state<Me | null>(null);
    let dashboard = $state<Dashboard | null>(null);
    let projects = $derived(dashboard?.projects ?? []);
    let error = $state("");
    let link_error = $state("");
    let dialog_open = $state(false);
    let help_open = $state(false);
    let dialog_mode = $state<"add" | "edit">("add");
    let editing_id = $state<number | null>(null);
    let editing = $derived(
        projects.find((p) => p.project_id === editing_id) ?? null,
    );
    let partner_invite = $state<PartnerInvite | null>(null);
    let invite = $state<(Invite & { token: string }) | null>(null);
    let join_first_time = $state(false);
    let joining = $state(false);
    let copied = $state(false);
    let regenerated = $state(false);
    let last_refresh = $state(Date.now());
    let now = $state(Date.now());
    let since = $derived(Math.max(0, Math.floor((now - last_refresh) / 1000)));

    let loaded = $derived(!!me && !!dashboard);
    let linked = $derived(me?.hackatime_linked ?? false);
    let active = $derived(
        projects.filter(
            (p) => !p.shipped && p.me.confirmed && p.friend?.confirmed,
        ),
    );
    let my_link_expired = $derived(dashboard?.hackatime === "expired");
    let expired_friends = $derived([
        ...new Map(
            active
                .filter((p) => p.friend?.hackatime === "expired")
                .map((p) => [p.friend!.user_id, p.friend!.name] as const),
        ),
    ]);
    let hackatime_down = $derived(
        dashboard?.hackatime === "unreachable" ||
            active.some(
                (p) =>
                    p.me.hackatime === "unreachable" ||
                    p.friend?.hackatime === "unreachable",
            ),
    );

    let refresh_seq = 0;
    let refresh_failed = $state(false);

    async function refresh() {
        const seq = ++refresh_seq;
        try {
            const result = await Promise.all([
                api<Me>("/dashboard/me"),
                api<Dashboard>("/dashboard/projects"),
            ]);
            if (seq !== refresh_seq) return;
            [me, dashboard] = result;
            last_refresh = Date.now();
            now = last_refresh;
            error = "";
            refresh_failed = false;
        } catch (e) {
            if (seq !== refresh_seq) return;
            error = (e as Error).message;
            refresh_failed = true;
        }
    }

    function read_link_error() {
        const code = new URLSearchParams(location.search).get(
            "hackatime_error",
        );
        if (!code) return;
        link_error = LINK_ERRORS[code] ?? LINK_ERRORS.failed;
        history.replaceState(null, "", "/dashboard");
    }

    function read_join_token() {
        return (
            document.cookie.match(/(?:^|;\s*)dyad_join=([0-9a-fA-F]{32})/)?.[1] ??
            null
        );
    }

    function clear_join_token() {
        document.cookie = "dyad_join=; path=/; max-age=0; secure; samesite=lax";
    }

    function is_client_error(e: unknown) {
        return e instanceof ApiError && e.status >= 400 && e.status < 500;
    }

    async function check_invite() {
        const token = read_join_token();
        if (!token) return;
        try {
            const found = await api<Invite>(`/dashboard/invite/${token}`);
            invite = { ...found, token };
            if (!found.project_name && linked)
                new_project({ name: found.name, token });
        } catch (e) {
            if (is_client_error(e)) clear_join_token();
            error = (e as Error).message;
        }
    }

    $effect(() => {
        const auto = setInterval(refresh, refresh_failed ? 3_000 : 30_000);
        return () => clearInterval(auto);
    });

    onMount(() => {
        const tick = setInterval(() => (now = Date.now()), 1000);
        read_link_error();
        refresh().then(check_invite);
        return () => {
            clearInterval(tick);
        };
    });

    async function join() {
        if (!invite) return;
        joining = true;
        try {
            await api("/dashboard/join", {
                invite_token: invite.token,
                first_time: join_first_time,
            });
            clear_join_token();
            invite = null;
            await refresh();
        } catch (e) {
            error = (e as Error).message;
            if (is_client_error(e)) {
                clear_join_token();
                invite = null;
            }
        } finally {
            joining = false;
        }
    }

    async function cancel_request(project_id: number) {
        try {
            await api("/dashboard/cancel-join", { project_id });
            await refresh();
        } catch (e) {
            error = (e as Error).message;
        }
    }

    function dismiss_invite() {
        clear_join_token();
        invite = null;
    }

    async function copy_invite() {
        if (!me) return;
        const link = `${location.origin}/join/${me.invite_token}`;
        try {
            await navigator.clipboard.writeText(link);
            copied = true;
            setTimeout(() => (copied = false), 2000);
        } catch {
            error = `Couldn't copy your invite link. Copy it from here: ${link}`;
        }
    }

    async function regenerate_invite() {
        if (
            !confirm(
                "Make a new invite link? Your old link will stop working.",
            )
        )
            return;
        try {
            const { invite_token } = await api<{ invite_token: string }>(
                "/dashboard/regenerate-invite",
                {},
            );
            if (me) me.invite_token = invite_token;
            regenerated = true;
            setTimeout(() => (regenerated = false), 2000);
        } catch (e) {
            error = (e as Error).message;
        }
    }

    async function set_reminders(box: HTMLInputElement) {
        const enabled = box.checked;
        try {
            await api("/dashboard/reminders", { enabled });
            if (me) me.reminders = enabled;
        } catch (e) {
            box.checked = !enabled;
            error = (e as Error).message;
        }
    }

    async function set_dms(box: HTMLInputElement) {
        const enabled = box.checked;
        try {
            await api("/dashboard/dms", { enabled });
            if (me) me.dms = enabled;
        } catch (e) {
            box.checked = !enabled;
            error = (e as Error).message;
        }
    }

    function new_project(partner: PartnerInvite | null = null) {
        dialog_mode = "add";
        editing_id = null;
        partner_invite = partner;
        dialog_open = true;
    }

    function edit_project(project: ProjectInfo) {
        dialog_mode = "edit";
        editing_id = project.project_id;
        dialog_open = true;
    }

    function on_saved(invite_token: string | null) {
        if (invite_token) {
            clear_join_token();
            if (invite?.token === invite_token) invite = null;
        }
        refresh();
    }

    function link_hackatime() {
        window.location.href = "/hackatime-login";
    }

    async function logout() {
        try {
            await api("/logout", {});
            window.location.href = "/";
        } catch (e) {
            error = (e as Error).message;
        }
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
    {partner_invite}
    onopen={refresh}
    onsaved={on_saved}
    onfailed={refresh}
/>

<HelpDialog bind:open={help_open} />

<div class="top-bar">
    <div class="container">
        <h1>Dyad Dashboard</h1>
        {#if me}
            <label class="friend-code reminders">
                <input
                    type="checkbox"
                    checked={me.dms}
                    onchange={(e) => set_dms(e.currentTarget)}
                />
                Slack me about important milestones with my projects
            </label>
            <label class="friend-code reminders">
                <input
                    type="checkbox"
                    checked={me.reminders}
                    disabled={!me.dms}
                    onchange={(e) => set_reminders(e.currentTarget)}
                />
                Slack me if I haven't worked on a project for a while
            </label>
            <p class="friend-code">
                <button class="link-button" onclick={refresh}>Refresh</button>
                · {since}s since last refresh ·
                <button class="link-button" onclick={logout}>Log out</button>
            </p>
        {/if}
    </div>
    {#if me}
        <div class="container friend-box">
            <p class="friend-code">
                Your friend code: <strong>{me.friend_code}</strong>
            </p>
            <p class="friend-code">
                <button class="link-button" onclick={copy_invite}
                    >{copied ? "Copied!" : "Copy invite link"}</button
                >
                ·
                <button class="link-button" onclick={regenerate_invite}
                    >{regenerated ? "New link ready!" : "New invite link"}</button
                >
            </p>
        </div>
    {/if}
    {#if loaded && linked}
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

{#if invite?.project_name}
    <div class="container banner invite">
        <span
            >{invite.name} invited you to join "{invite.project_name}". They'll
            need to approve your request.</span
        >
        {#if linked}
            <label class="reminders">
                <input type="checkbox" bind:checked={join_first_time} />
                My first time coding / doing a YSWS
            </label>
            <button
                class="banner-button primary"
                disabled={joining}
                onclick={join}>Request to join</button
            >
        {:else}
            <button class="banner-button primary" onclick={link_hackatime}
                >Link Hackatime to continue</button
            >
        {/if}
        <button
            class="banner-button"
            disabled={joining}
            onclick={dismiss_invite}>Dismiss</button
        >
    </div>
{:else if invite}
    {@const partner = { name: invite.name, token: invite.token }}
    <div class="container banner invite">
        <span>{invite.name} invited you to start a project together.</span>
        {#if linked}
            <button
                class="banner-button primary"
                onclick={() => new_project(partner)}>Add project</button
            >
        {:else}
            <button class="banner-button primary" onclick={link_hackatime}
                >Link Hackatime to continue</button
            >
        {/if}
        <button class="banner-button" onclick={dismiss_invite}>Dismiss</button>
    </div>
{/if}

{#if error}
    <div class="container banner">{error}</div>
{/if}

{#if link_error}
    <div class="container banner">{link_error}</div>
{/if}

{#if my_link_expired}
    <div class="container banner invite">
        <span
            >Your Hackatime connection stopped working, so your hours can't be
            checked.</span
        >
        <button class="banner-button primary" onclick={link_hackatime}
            >Relink Hackatime</button
        >
    </div>
{/if}

{#each expired_friends as [user_id, name] (user_id)}
    <div class="container banner">
        {name}'s Hackatime connection stopped working, so their hours can't be
        checked. Ask them to relink it from their Dyad dashboard.
    </div>
{/each}

{#if hackatime_down}
    <div class="container banner">
        Couldn't reach Hackatime, so some hours are missing. Try again in a bit.
    </div>
{/if}

{#each dashboard?.join_requests ?? [] as request (request.project_id)}
    <div class="container banner invite">
        <span
            >You asked to join "{request.project_name}" with {request.owner_name}.
            Waiting for them to approve.</span
        >
        <button
            class="banner-button"
            onclick={() => cancel_request(request.project_id)}>Cancel</button
        >
    </div>
{/each}

<div class="container project-container themed-scroll">
    {#if loaded && linked}
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
    {:else if loaded}
        <div class="link-hackatime-button-container">
            <button class="link-hackatime-button" onclick={link_hackatime}
                >Link Hackatime to continue</button
            >
        </div>
    {/if}
</div>

<button class="help-button container" onclick={() => (help_open = true)}
    >Help</button
>

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
    .friend-box {
        display: flex;
        flex-direction: column;
        justify-content: center;
        gap: 8px;
        text-align: center;
    }
    .new-project-button {
        color: var(--text-primary);
        transition: background-color 0.3s ease;
    }
    .new-project-button:hover {
        background-color: var(--accent-primary);
        color: var(--text-on-accent);
    }
    .help-button {
        position: fixed;
        right: 24px;
        bottom: 24px;
        padding: 8px 20px;
        color: var(--text-primary);
        font-family: inherit;
        font-size: 1.1rem;
        cursor: pointer;
        transition: background-color 0.3s ease;
    }
    .help-button:hover {
        background-color: var(--accent-primary);
        color: var(--text-on-accent);
    }
    .friend-code {
        margin: 0;
        color: var(--text-secondary);
    }
    .banner a {
        color: var(--accent-text);
    }
    .link-button {
        padding: 0;
        background: none;
        border: none;
        color: var(--accent-text);
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
        color: var(--text-on-accent);
    }
    .banner-button:hover:not(:disabled) {
        background-color: var(--accent-secondary);
        color: var(--text-on-accent);
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
        color: var(--text-on-accent);
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
