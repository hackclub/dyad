<script lang="ts">
    import "$lib/palette.css";
    import "$lib/scrollbar.css";
    import { untrack } from "svelte";
    import { cubicOut } from "svelte/easing";
    import {
        api,
        type GameCatalog,
        type HackatimeProject,
        type PartnerInvite,
        type ProjectInfo,
    } from "$lib/api";
    import FriendCodeInput from "./FriendCodeInput.svelte";
    import GamesPane from "./GamesPane.svelte";

    type Mode = "add" | "edit";

    interface Project {
        name: string;
        description: string;
        screenshot_url: string;
        code_url: string;
        playable_url: string;
    }

    interface Field {
        key: keyof Project;
        label: string;
        kind: "text" | "url" | "multiline";
        helper: string;
        max: number;
        required?: boolean;
    }

    interface Props {
        open?: boolean;
        mode?: Mode;
        existing?: ProjectInfo | null;
        partner_invite?: PartnerInvite | null;
        onsaved?: (invite_token: string | null) => void;
        onfailed?: () => void;
    }

    let {
        open = $bindable(false),
        mode = "add",
        existing = null,
        partner_invite = null,
        onsaved,
        onfailed,
    }: Props = $props();

    const HEADINGS: Record<
        Mode,
        { title: string; blurb: string; confirm: string }
    > = {
        add: {
            title: "Add Project",
            blurb: "Start tracking a new project.",
            confirm: "Add Project",
        },
        edit: {
            title: "Edit Project",
            blurb: "Update the details of the project you're working on.",
            confirm: "Save Changes",
        },
    };

    const FIELDS: Field[] = [
        {
            key: "name",
            label: "Name",
            kind: "text",
            required: true,
            max: 100,
            helper: "Your project's name.",
        },
        {
            key: "description",
            label: "Description",
            kind: "multiline",
            max: 2000,
            helper: "A short summary of what the two of you are building.",
        },
        {
            key: "screenshot_url",
            label: "Screenshot URL",
            kind: "url",
            max: 500,
            helper: "A link to a screenshot of your project.",
        },
        {
            key: "code_url",
            label: "Code URL",
            kind: "url",
            max: 500,
            helper: "The link to your source code, like a GitHub repo.",
        },
        {
            key: "playable_url",
            label: "Playable URL",
            kind: "url",
            max: 500,
            helper: "A link where people can try your project. This can be something like a GitHub Release link too.",
        },
    ];

    let visible_fields = $derived(
        mode === "add" ? FIELDS.filter((f) => f.key === "name") : FIELDS,
    );

    function details_of(p: ProjectInfo | null): Project {
        return {
            name: p?.name ?? "",
            description: p?.description ?? "",
            screenshot_url: p?.screenshot_url ?? "",
            code_url: p?.code_url ?? "",
            playable_url: p?.playable_url ?? "",
        };
    }

    let project = $state<Project>(details_of(null));
    let project_id: number | null = null;
    let generation = 0;
    let friend_code = $state("");
    let games = $state<string[]>([]);
    let first_time = $state(false);
    let selected_hackatime = $state<string[]>([]);
    let catalog = $state<GameCatalog>({ credit_budget: 0, games: [] });
    let hackatime_projects = $state<HackatimeProject[]>([]);
    let hackatime_loaded = $state(false);
    let hackatime_error = $state("");
    let hackatime_filter = $state("");
    let missing_hackatime = $derived(
        selected_hackatime.filter(
            (n) => !hackatime_projects.some((ht) => ht.name === n),
        ),
    );
    let details_locked = $derived(
        mode === "edit" &&
            (!!existing?.friend?.ready_to_ship ||
                !!existing?.friend?.submitting),
    );
    let filtered_hackatime = $derived(
        hackatime_projects.filter((ht) =>
            ht.name
                .toLowerCase()
                .includes(hackatime_filter.trim().toLowerCase()),
        ),
    );
    let error = $state("");
    let saving = $state(false);
    let games_open = $state(false);

    let dialog: HTMLDialogElement;
    let pane: HTMLDivElement;
    let pressed_outside = false;
    let trigger = $state<HTMLButtonElement>();
    let hackatime_list = $state<HTMLDivElement>();
    let more_below = $state(false);

    const reduced_motion = () =>
        matchMedia("(prefers-reduced-motion: reduce)").matches;

    function reset() {
        generation++;
        saving = false;
        project_id = existing?.project_id ?? null;
        project = details_of(existing);
        friend_code = "";
        games = [...(existing?.games ?? [])];
        first_time = existing?.me.first_time ?? false;
        selected_hackatime = [
            ...new Set(existing?.hackatime_project_names ?? []),
        ];
        hackatime_filter = "";
        hackatime_projects = [];
        hackatime_loaded = false;
        hackatime_error = "";
        error = "";
        games_open = false;
    }

    function flow_out(node: Element) {
        if (!dialog.open) return { duration: 0 };
        const slot = node.getBoundingClientRect();
        const from = trigger?.getBoundingClientRect();
        const y = from
            ? (from.top + from.bottom) / 2 - slot.top
            : slot.height / 2;
        return {
            duration: reduced_motion() ? 0 : 500,
            easing: cubicOut,
            css: (t: number, u: number) =>
                `clip-path: inset(${y * u}px ${u * 100}% ${(slot.height - y) * u}px 0 round 16px); opacity: ${0.3 + 0.7 * t};`,
        };
    }

    function slide(direction: "in" | "out") {
        const offscreen = `translateX(${-(pane.offsetLeft + pane.offsetWidth)}px)`;
        const frames = [{ transform: offscreen }, { transform: "none" }];
        const timing: KeyframeAnimationOptions = {
            duration: reduced_motion() ? 0 : 300,
            direction: direction === "in" ? "normal" : "reverse",
        };
        dialog.animate([{ backgroundColor: "transparent" }, {}], timing);
        return pane.animate(frames, {
            ...timing,
            easing: "cubic-bezier(0.33, 1, 0.68, 1)",
        });
    }

    function on_outside_click(event: MouseEvent) {
        if (pressed_outside && event.target === dialog) open = false;
        pressed_outside = false;
    }

    function on_cancel(event: Event) {
        event.preventDefault();
        if (games_open) games_open = false;
        else open = false;
    }

    function load_options() {
        api<GameCatalog>("/dashboard/games")
            .then((c) => (catalog = c))
            .catch((e) => (error = (e as Error).message));
        api<HackatimeProject[]>("/dashboard/projects-list")
            .then((p) => {
                hackatime_projects = [
                    ...new Map(p.map((ht) => [ht.name, ht])).values(),
                ];
                hackatime_loaded = true;
            })
            .catch((e) => (hackatime_error = (e as Error).message));
    }

    function toggle_hackatime(name: string, checked: boolean) {
        selected_hackatime = selected_hackatime.filter((n) => n !== name);
        if (checked) selected_hackatime.push(name);
    }

    $effect(() => {
        if (!details_locked || !existing) return;
        const locked = existing;
        untrack(() => {
            project = details_of(locked);
            games = [...locked.games];
        });
    });

    $effect(() => {
        if (!dialog) return;
        if (open && !dialog.open) {
            untrack(() => {
                reset();
                if (mode === "edit") load_options();
            });
            dialog.showModal();
            slide("in");
        } else if (!open && dialog.open) {
            games_open = false;
            slide("out").finished.then(() => !open && dialog.close());
        }
    });

    function update_fade() {
        if (!hackatime_list) return;
        more_below =
            hackatime_list.scrollHeight -
                hackatime_list.scrollTop -
                hackatime_list.clientHeight >
            1;
    }

    $effect(() => {
        void [filtered_hackatime, open];
        requestAnimationFrame(update_fade);
    });

    async function handle_submit(event: SubmitEvent) {
        event.preventDefault();
        const invite = mode === "add" ? partner_invite : null;
        if (mode === "add" && !invite && friend_code && friend_code.length !== 6) {
            error =
                "Enter all six digits of your partner's code, or leave it blank";
            return;
        }
        saving = true;
        error = "";
        const current = generation;
        try {
            await api("/dashboard/upsert-project", {
                project_id: mode === "edit" ? project_id : null,
                friend_code:
                    mode === "add" && !invite && friend_code
                        ? Number(friend_code)
                        : null,
                invite_token: invite?.token ?? null,
                ...project,
                games,
                first_time,
                hackatime_project_names: selected_hackatime,
            });
            if (current === generation) open = false;
            onsaved?.(invite?.token ?? null);
        } catch (e) {
            if (current === generation) error = (e as Error).message;
            onfailed?.();
        } finally {
            if (current === generation) saving = false;
        }
    }
</script>

<dialog
    bind:this={dialog}
    class="project-dialog"
    onclose={() => {
        open = false;
        games_open = false;
    }}
    oncancel={on_cancel}
    onpointerdown={(e) => (pressed_outside = e.target === dialog)}
    onclick={on_outside_click}
>
    <div class="project-pane themed-scroll" bind:this={pane}>
        <h2 class="dialog-title">{HEADINGS[mode].title}</h2>

        <p class="dialog-blurb">{HEADINGS[mode].blurb}</p>

        <form class="dialog-form" onsubmit={handle_submit}>
            {#if details_locked}
                <p class="field-helper">
                    {existing?.friend?.name} is ready to ship, so the project details
                    are locked. You can still change your Hackatime projects and
                    first-time setting.
                </p>
            {/if}
            {#each visible_fields as field (field.key)}
                <label class="field">
                    <span class="field-name">{field.label}</span>
                    <span class="field-helper">{field.helper}</span>
                    {#if field.kind === "multiline"}
                        <textarea
                            bind:value={project[field.key]}
                            maxlength={field.max}
                            rows="4"
                            disabled={details_locked}
                        ></textarea>
                    {:else if field.kind === "url"}
                        <input
                            type="url"
                            bind:value={project[field.key]}
                            maxlength={field.max}
                            placeholder="https://"
                            disabled={details_locked}
                        />
                    {:else}
                        <input
                            type="text"
                            bind:value={project[field.key]}
                            maxlength={field.max}
                            required={field.required}
                            disabled={details_locked}
                        />
                    {/if}
                </label>
            {/each}

            {#if mode === "add" && partner_invite}
                <div class="field">
                    <span class="field-name">Partner</span>
                    <span class="field-helper"
                        >{partner_invite.name} (from their invite link). They'll
                        need to accept the invite.</span
                    >
                </div>
            {:else if mode === "add"}
                <div
                    class="field"
                    role="group"
                    aria-labelledby="friend-code-label"
                >
                    <span class="field-name" id="friend-code-label"
                        >Partner's Friend Code</span
                    >
                    <span class="field-helper"
                        >Optional. Your partner's six-digit code; they'll need
                        to accept the invite. Leave it blank to send them your
                        invite link instead.</span
                    >
                    <FriendCodeInput bind:value={friend_code} />
                </div>
            {:else}
                <div class="field">
                    <span class="field-name">Games</span>
                    <span class="field-helper"
                        >The two-player games you'd like, paid for with your
                        credits.</span
                    >
                    <button
                        type="button"
                        bind:this={trigger}
                        class="games-trigger"
                        class:active={games_open}
                        aria-expanded={games_open}
                        disabled={details_locked}
                        onclick={() => (games_open = !games_open)}
                    >
                        <span class="games-trigger-text">
                            {games.length ? games.join(", ") : "Pick games"}
                        </span>
                        <span class="trigger-icon"
                            >{games_open ? "−" : "+"}</span
                        >
                    </button>
                </div>

                <div class="field">
                    <span class="field-name">Your Hackatime Projects</span>
                    <span class="field-helper"
                        >The Hackatime projects that count toward this one.</span
                    >
                    <div>
                        <div
                            class="hackatime-list themed-scroll"
                            class:more-below={more_below}
                            bind:this={hackatime_list}
                            onscroll={update_fade}
                        >
                            {#each missing_hackatime as name (name)}
                                <label class="check">
                                    <input
                                        type="checkbox"
                                        checked
                                        onchange={(e) =>
                                            toggle_hackatime(
                                                name,
                                                e.currentTarget.checked,
                                            )}
                                    />
                                    {name}
                                    {#if hackatime_loaded}
                                        <span class="field-helper"
                                            >(not found on Hackatime)</span
                                        >
                                    {/if}
                                </label>
                            {/each}
                            {#each filtered_hackatime as ht (ht.name)}
                                {@const chosen = selected_hackatime.includes(
                                    ht.name,
                                )}
                                {@const blocked = !!ht.unusable && !chosen}
                                <label class="check">
                                    <input
                                        type="checkbox"
                                        checked={chosen}
                                        disabled={blocked}
                                        onchange={(e) =>
                                            toggle_hackatime(
                                                ht.name,
                                                e.currentTarget.checked,
                                            )}
                                    />
                                    <span>
                                        <span class:blocked
                                            >{ht.name}
                                            <span class="field-helper"
                                                >({ht.hours}h)</span
                                            ></span
                                        >
                                        {#if ht.unusable}
                                            <span class="field-helper reason"
                                                >{ht.unusable}</span
                                            >
                                        {/if}
                                    </span>
                                </label>
                            {:else}
                                <span class="field-helper"
                                    >{hackatime_error ||
                                        (!hackatime_loaded
                                            ? "Loading..."
                                            : hackatime_projects.length
                                              ? "No matching projects."
                                              : "No Hackatime projects found.")}</span
                                >
                            {/each}
                        </div>
                        <input
                            class="hackatime-filter"
                            type="search"
                            placeholder="Filter projects"
                            aria-label="Filter Hackatime projects"
                            bind:value={hackatime_filter}
                            onkeydown={(e) =>
                                e.key === "Enter" && e.preventDefault()}
                        />
                    </div>
                </div>
            {/if}

            <label class="check">
                <input type="checkbox" bind:checked={first_time} />
                This is my first time coding or doing a Hack Club YSWS
            </label>

            {#if error}
                <p class="dialog-error">{error}</p>
            {/if}

            <div class="dialog-actions">
                <button
                    type="button"
                    class="cancel-button"
                    onclick={() => (open = false)}>Cancel</button
                >
                <button type="submit" class="confirm-button" disabled={saving}
                    >{HEADINGS[mode].confirm}</button
                >
            </div>
        </form>
    </div>

    {#if games_open}
        <div class="games-slot" transition:flow_out>
            <GamesPane
                {catalog}
                bind:selected={games}
                ondone={() => (games_open = false)}
            />
        </div>
    {/if}
</dialog>

<style>
    .project-dialog {
        --gap: 24px;
        box-sizing: border-box;
        position: fixed;
        inset: 0;
        width: 100vw;
        height: 100dvh;
        max-width: none;
        max-height: none;
        margin: 0;
        padding: var(--gap);
        background: rgb(17 10 15 / 0.6);
        border: none;
        color: var(--text-secondary);
        font-family: inherit;
        overflow: hidden;
    }
    .project-dialog[open] {
        display: flex;
        gap: var(--gap);
    }
    .project-dialog::backdrop {
        background: transparent;
    }
    .project-pane {
        box-sizing: border-box;
        flex: 0 0 calc((100% - var(--gap)) / 3);
        min-width: 340px;
        height: 100%;
        overflow-y: auto;
        padding: 16px;
        background-color: var(--bg-tertiary);
        border: 4px solid var(--border);
        border-radius: 16px;
    }
    .games-slot {
        flex: 1;
        min-width: 0;
        height: 100%;
    }
    @media (max-width: 800px) {
        .project-dialog {
            --gap: 16px;
        }
        .project-pane {
            flex-basis: 100%;
            min-width: 0;
        }
        .games-slot {
            position: absolute;
            inset: var(--gap);
        }
    }

    .dialog-title {
        margin: 0;
        color: var(--text-primary);
        font-size: 1.75rem;
        font-weight: 600;
    }
    .dialog-blurb {
        margin: 8px 0 16px;
        color: var(--text-secondary);
        font-size: 0.95rem;
    }

    .dialog-form {
        display: flex;
        flex-direction: column;
        gap: 16px;
    }
    .field {
        display: flex;
        flex-direction: column;
        gap: 4px;
    }
    .field-name {
        color: var(--text-primary);
        font-size: 1.1rem;
    }
    .field-helper {
        color: var(--text-secondary);
        font-size: 0.85rem;
        opacity: 0.8;
    }
    .field input:not([type="checkbox"]),
    .field textarea {
        box-sizing: border-box;
        width: 100%;
        margin-top: 4px;
        padding: 8px;
        background-color: var(--bg-secondary);
        color: var(--text-secondary);
        border: 3px solid var(--border);
        border-radius: 6px;
        font-family: inherit;
        font-size: 1rem;
        resize: vertical;
    }
    .field input:not([type="checkbox"]):focus,
    .field textarea:focus {
        outline: 3px solid var(--accent-primary);
        outline-offset: -3px;
    }

    .games-trigger {
        box-sizing: border-box;
        display: flex;
        align-items: center;
        gap: 8px;
        width: 100%;
        margin-top: 4px;
        padding: 8px;
        background-color: var(--bg-secondary);
        color: var(--text-secondary);
        border: 3px solid var(--border);
        border-radius: 6px;
        font-family: inherit;
        font-size: 1rem;
        text-align: left;
        cursor: pointer;
    }
    .games-trigger:disabled,
    .field input:disabled,
    .field textarea:disabled {
        opacity: 0.5;
        cursor: default;
    }
    .games-trigger:focus-visible,
    .games-trigger.active {
        outline: 3px solid var(--accent-primary);
        outline-offset: -3px;
    }
    .games-trigger-text {
        flex: 1;
        overflow: hidden;
        white-space: nowrap;
        text-overflow: ellipsis;
    }
    .trigger-icon {
        font-size: 1.2rem;
        font-weight: bold;
        line-height: 1;
    }

    .hackatime-list {
        --scroll-thumb: var(--bg-tertiary);
        display: flex;
        flex-direction: column;
        gap: 4px;
        max-height: 160px;
        overflow-y: auto;
        padding: 8px;
        background-color: var(--bg-secondary);
        border: 3px solid var(--border);
        border-bottom-width: 0;
        border-radius: 6px 6px 0 0;
    }
    .hackatime-list::after {
        content: "";
        position: sticky;
        bottom: 0;
        flex-shrink: 0;
        height: 1.5em;
        margin-top: calc(-1.5em - 4px);
        background: linear-gradient(transparent, var(--bg-secondary));
        pointer-events: none;
        opacity: 0;
        transition: opacity 0.2s ease;
    }
    .hackatime-list.more-below::after {
        opacity: 1;
    }
    .field input.hackatime-filter {
        display: block;
        margin-top: 0;
        border-radius: 0 0 6px 6px;
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
    .blocked {
        opacity: 0.5;
    }
    .reason {
        display: block;
    }
    .dialog-error {
        margin: 0;
        color: var(--accent-text);
    }

    .dialog-actions {
        display: flex;
        justify-content: flex-end;
        gap: 8px;
        margin-top: 8px;
    }
    .dialog-actions button {
        padding: 8px 16px;
        border: 4px solid var(--border);
        border-radius: 6px;
        color: var(--text-secondary);
        font-family: inherit;
        font-size: 1rem;
        cursor: pointer;
        transition: background-color 0.3s ease;
    }
    .cancel-button {
        background-color: var(--bg-secondary);
    }
    .cancel-button:hover {
        background-color: var(--bg-tertiary);
    }
    .dialog-actions .confirm-button {
        background-color: var(--accent-primary);
        color: var(--text-on-accent);
    }
    .dialog-actions .confirm-button:hover {
        background-color: var(--accent-secondary);
    }
</style>
