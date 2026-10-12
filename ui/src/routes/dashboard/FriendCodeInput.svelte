<script lang="ts">
    import "$lib/palette.css";
    import { untrack } from "svelte";

    const LENGTH = 6;

    let { value = $bindable("") }: { value?: string } = $props();

    let digits = $state<string[]>(Array(LENGTH).fill(""));
    let boxes: HTMLInputElement[] = [];

    $effect(() => {
        if (value !== untrack(() => digits.join(""))) {
            digits = Array.from({ length: LENGTH }, (_, i) => value[i] ?? "");
        }
    });

    function focus(i: number) {
        boxes[Math.max(0, Math.min(i, LENGTH - 1))]?.focus();
    }

    function set(i: number, digit: string) {
        digits[i] = digit;
        value = digits.join("");
    }

    function fill(start: number, text: string) {
        const incoming = text.replace(/\D/g, "").slice(0, LENGTH - start);
        [...incoming].forEach((d, k) => (digits[start + k] = d));
        value = digits.join("");
        focus(start + incoming.length);
    }

    function onkeydown(i: number, e: KeyboardEvent) {
        if (e.ctrlKey || e.metaKey || e.altKey) return;
        if (/^\d$/.test(e.key)) {
            e.preventDefault();
            set(i, e.key);
            focus(i + 1);
        } else if (e.key === "Backspace") {
            e.preventDefault();
            if (digits[i]) set(i, "");
            else if (i > 0) {
                set(i - 1, "");
                focus(i - 1);
            }
        } else if (e.key === "ArrowLeft") {
            e.preventDefault();
            focus(i - 1);
        } else if (e.key === "ArrowRight") {
            e.preventDefault();
            focus(i + 1);
        } else if (e.key.length === 1) {
            e.preventDefault();
        }
    }

    function oninput(i: number, e: Event) {
        const box = e.currentTarget as HTMLInputElement;
        const typed = box.value.replace(/\D/g, "");
        if ((e as InputEvent).inputType?.startsWith("delete")) {
            set(i, typed.slice(0, 1));
            box.value = digits[i];
            return;
        }
        box.value = digits[i];
        if (typed.length > 1) fill(i, typed);
        else if (typed) {
            set(i, typed);
            focus(i + 1);
        }
    }

    function onpaste(i: number, e: ClipboardEvent) {
        e.preventDefault();
        fill(i, e.clipboardData?.getData("text") ?? "");
    }
</script>

<div class="code-boxes">
    {#each digits as digit, i (i)}
        <input
            bind:this={boxes[i]}
            type="text"
            inputmode="numeric"
            autocomplete="off"
            pattern="[0-9]"
            aria-label={`Digit ${i + 1}`}
            value={digit}
            onkeydown={(e) => onkeydown(i, e)}
            oninput={(e) => oninput(i, e)}
            onpaste={(e) => onpaste(i, e)}
            onfocus={(e) => e.currentTarget.select()}
        />
    {/each}
</div>

<style>
    .code-boxes {
        display: flex;
        gap: clamp(4px, 2vw, 8px);
        margin-top: 4px;
    }
    .code-boxes input {
        box-sizing: border-box;
        flex: 0 1 48px;
        min-width: 0;
        height: 56px;
        padding: 0;
        background-color: var(--bg-secondary);
        color: var(--text-primary);
        border: 3px solid var(--border);
        border-radius: 6px;
        font-family: inherit;
        font-size: 1.5rem;
        font-weight: bold;
        text-align: center;
        caret-color: transparent;
    }
    .code-boxes input:focus {
        outline: 3px solid var(--accent-primary);
        outline-offset: -3px;
    }
</style>
