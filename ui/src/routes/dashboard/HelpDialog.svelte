<script lang="ts">
    import "$lib/palette.css";
    import "$lib/scrollbar.css";
    import { marked } from "marked";
    import guide from "./help.md?raw";

    let { open = $bindable(false) }: { open?: boolean } = $props();

    const html = marked.parse(guide, { async: false, gfm: true });

    let dialog: HTMLDialogElement;
    let pane: HTMLDivElement;
    let pressed_outside = false;

    const reduced_motion = () =>
        window.matchMedia("(prefers-reduced-motion: reduce)").matches;

    function slide(direction: "in" | "out") {
        const timing: KeyframeAnimationOptions = {
            duration: reduced_motion() ? 0 : 250,
            direction: direction === "in" ? "normal" : "reverse",
            easing: "cubic-bezier(0.33, 1, 0.68, 1)",
        };
        dialog.animate([{ backgroundColor: "transparent" }, {}], timing);
        return pane.animate(
            [{ transform: "translateY(40px)", opacity: 0 }, { transform: "none", opacity: 1 }],
            timing,
        );
    }

    $effect(() => {
        if (!dialog) return;
        if (open && !dialog.open) {
            dialog.showModal();
            pane.scrollTop = 0;
            slide("in");
        } else if (!open && dialog.open) {
            slide("out").finished.then(() => !open && dialog.close());
        }
    });

    function on_outside_click(event: MouseEvent) {
        if (pressed_outside && event.target === dialog) open = false;
        pressed_outside = false;
    }
</script>

<dialog
    bind:this={dialog}
    class="help-dialog"
    onclose={() => (open = false)}
    oncancel={(e) => {
        e.preventDefault();
        open = false;
    }}
    onpointerdown={(e) => (pressed_outside = e.target === dialog)}
    onclick={on_outside_click}
>
    <div class="help-pane themed-scroll" bind:this={pane}>
        <div class="help-content">
            {@html html}
        </div>
        <div class="dialog-actions">
            <button type="button" class="close-button" onclick={() => (open = false)}
                >Close</button
            >
        </div>
    </div>
</dialog>

<style>
    .help-dialog {
        box-sizing: border-box;
        position: fixed;
        inset: 0;
        width: 100vw;
        height: 100dvh;
        max-width: none;
        max-height: none;
        margin: 0;
        padding: 24px;
        background: rgb(17 10 15 / 0.6);
        border: none;
        color: var(--text-secondary);
        font-family: inherit;
        overflow: hidden;
    }
    .help-dialog[open] {
        display: flex;
        align-items: center;
        justify-content: center;
    }
    .help-dialog::backdrop {
        background: transparent;
    }
    .help-pane {
        box-sizing: border-box;
        width: min(720px, 100%);
        max-height: 100%;
        overflow-y: auto;
        padding: 24px;
        background-color: var(--bg-tertiary);
        border: 4px solid var(--border);
        border-radius: 16px;
    }

    .help-content {
        color: var(--text-secondary);
        font-size: 1.05rem;
        line-height: 1.55;
    }
    .help-content :global(h1) {
        margin: 0 0 16px;
        color: var(--text-primary);
        font-size: 1.75rem;
        font-weight: 600;
    }
    .help-content :global(h2) {
        margin: 28px 0 8px;
        color: var(--text-primary);
        font-size: 1.3rem;
        font-weight: 600;
    }
    .help-content :global(h3) {
        margin: 20px 0 4px;
        color: var(--text-primary);
        font-size: 1.1rem;
        font-weight: 600;
    }
    .help-content :global(p) {
        margin: 0 0 12px;
    }
    .help-content :global(ul),
    .help-content :global(ol) {
        margin: 0 0 12px;
        padding-left: 1.5em;
    }
    .help-content :global(li) {
        margin-bottom: 4px;
    }
    .help-content :global(a) {
        color: var(--accent-text);
    }
    .help-content :global(strong) {
        color: var(--text-primary);
    }
    .help-content :global(code) {
        padding: 1px 5px;
        background-color: var(--bg-secondary);
        border-radius: 4px;
        font-size: 0.9em;
    }
    .help-content :global(pre) {
        margin: 0 0 12px;
        padding: 12px;
        background-color: var(--bg-secondary);
        border: 3px solid var(--border);
        border-radius: 6px;
        overflow-x: auto;
    }
    .help-content :global(pre code) {
        padding: 0;
        background: none;
    }
    .help-content :global(blockquote) {
        margin: 0 0 12px;
        padding: 8px 16px;
        border-left: 4px solid var(--accent-primary);
        background-color: rgb(17 10 15 / 0.2);
        border-radius: 0 6px 6px 0;
    }
    .help-content :global(blockquote p:last-child) {
        margin-bottom: 0;
    }
    .help-content :global(hr) {
        margin: 20px 0;
        border: none;
        border-top: 3px solid var(--border);
    }
    .help-content :global(img) {
        max-width: 100%;
        border: 3px solid var(--border);
        border-radius: 6px;
    }
    .help-content :global(table) {
        margin: 0 0 12px;
        border-collapse: collapse;
    }
    .help-content :global(th),
    .help-content :global(td) {
        padding: 6px 10px;
        border: 2px solid var(--border);
        text-align: left;
    }
    .help-content :global(th) {
        background-color: var(--bg-secondary);
        color: var(--text-primary);
    }

    .dialog-actions {
        display: flex;
        justify-content: flex-end;
        margin-top: 16px;
    }
    .close-button {
        padding: 8px 16px;
        background-color: var(--bg-secondary);
        border: 4px solid var(--border);
        border-radius: 6px;
        color: var(--text-secondary);
        font-family: inherit;
        font-size: 1rem;
        cursor: pointer;
        transition: background-color 0.3s ease;
    }
    .close-button:hover {
        background-color: var(--accent-primary);
        color: var(--text-on-accent);
    }
</style>
