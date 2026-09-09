<script lang="ts">
  import "$lib/palette.css";

  type Mode = "add" | "edit";

  interface Project {
    name: string;
    description: string;
    screenshot_url: string;
  }

  interface Field {
    key: keyof Project;
    label: string;
    kind: "text" | "url" | "multiline";
    helper: Record<Mode, string>;
    required?: boolean;
  }

  interface Props {
    open?: boolean;
    onsubmit?: (project: Project) => void;
  }

  let { open = $bindable(false), onsubmit }: Props = $props();

  const HEADINGS: Record<Mode, { title: string; blurb: string; confirm: string }> =
    {
      add: {
        title: "Add Project",
        blurb: "Tell us about the project you and your friend are starting.",
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
      helper: {
        add: "What are you calling it?",
        edit: "Rename your project.",
      },
    },
    {
      key: "description",
      label: "Description",
      kind: "multiline",
      helper: {
        add: "What are the two of you building?",
        edit: "Change what you're building.",
      },
    },
    {
      key: "screenshot_url",
      label: "Screenshot URL",
      kind: "url",
      helper: {
        add: "Link a screenshot of your project.",
        edit: "Replace the screenshot link.",
      },
    },
  ];

  let mode = $state<Mode>("add");
  let project = $state<Project>({
    name: "",
    description: "",
    screenshot_url: "",
  });

  let dialog: HTMLDialogElement;

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  });

  function handle_submit(event: SubmitEvent) {
    event.preventDefault();
    onsubmit?.({ ...project });
    open = false;
  }
</script>

<dialog bind:this={dialog} class="project-dialog" onclose={() => (open = false)}>
  <div class="dialog-head">
    <h2 class="dialog-title">{HEADINGS[mode].title}</h2>
    <div class="mode-toggle">
      <span class="mode-label" class:on={mode === "add"}>Add</span>
      <button
        type="button"
        class="toggle-track"
        class:edit={mode === "edit"}
        role="switch"
        aria-checked={mode === "edit"}
        aria-label="Switch between add and edit wording"
        onclick={() => (mode = mode === "add" ? "edit" : "add")}
      >
        <span class="toggle-knob"></span>
      </button>
      <span class="mode-label" class:on={mode === "edit"}>Edit</span>
    </div>
  </div>

  <p class="dialog-blurb">{HEADINGS[mode].blurb}</p>

  <form class="dialog-form" onsubmit={handle_submit}>
    {#each FIELDS as field (field.key)}
      <label class="field">
        <span class="field-name">{field.label}</span>
        <span class="field-helper">{field.helper[mode]}</span>
        {#if field.kind === "multiline"}
          <textarea bind:value={project[field.key]} rows="4"></textarea>
        {:else if field.kind === "url"}
          <input
            type="url"
            bind:value={project[field.key]}
            placeholder="https://"
          />
        {:else}
          <input
            type="text"
            bind:value={project[field.key]}
            required={field.required}
          />
        {/if}
      </label>
    {/each}

    <div class="dialog-actions">
      <button type="button" class="cancel-button" onclick={() => (open = false)}
        >Cancel</button
      >
      <button type="submit" class="confirm-button">{HEADINGS[mode].confirm}</button>
    </div>
  </form>
</dialog>

<style>
  .project-dialog {
    box-sizing: border-box;
    margin: auto;
    width: min(520px, 90vw);
    padding: 16px;
    background-color: var(--bg-tertiary);
    border: 4px solid var(--border);
    border-radius: 16px;
    color: var(--text-secondary);
    font-family: inherit;
  }
  .project-dialog::backdrop {
    background-color: rgb(17 10 15 / 0.6);
  }

  .dialog-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 16px;
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

  .mode-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .mode-label {
    color: var(--text-secondary);
    font-size: 0.9rem;
    opacity: 0.55;
    transition: opacity 0.2s ease;
  }
  .mode-label.on {
    color: var(--text-primary);
    opacity: 1;
  }
  .toggle-track {
    box-sizing: border-box;
    width: 52px;
    height: 28px;
    padding: 0;
    background-color: var(--bg-secondary);
    border: 3px solid var(--border);
    border-radius: 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    transition: background-color 0.2s ease;
  }
  .toggle-track.edit {
    background-color: var(--accent-primary);
  }
  .toggle-knob {
    width: 16px;
    height: 16px;
    margin-left: 3px;
    background-color: var(--accent-secondary);
    border-radius: 50%;
    transition: transform 0.2s ease;
  }
  .toggle-track.edit .toggle-knob {
    background-color: var(--bg-secondary);
    transform: translateX(24px);
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
  .field input,
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
  .field input:focus,
  .field textarea:focus {
    outline: 3px solid var(--accent-primary);
    outline-offset: -3px;
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
  .confirm-button {
    background-color: var(--accent-primary);
  }
  .confirm-button:hover {
    background-color: var(--accent-secondary);
  }
</style>
