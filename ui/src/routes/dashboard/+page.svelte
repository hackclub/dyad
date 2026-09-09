<script lang="ts">
  import "$lib/palette.css";
  import ProjectEntry from "./ProjectEntry.svelte";
  import ProjectDialog from "./ProjectDialog.svelte";
  import { onMount } from "svelte";

  let linked = $state(true);
  let dialog_open = $state(false);
  onMount(async () => {
    const res = await fetch("/api/hackatime-linked");
    linked = await res.json();
  });

  function new_project() {
    dialog_open = true;
  }

  function link_hackatime() {
    window.location.href = "/hackatime-login";
  }
</script>

<ProjectDialog bind:open={dialog_open} />

<div class="top-bar">
  <div class="container">
    <h1>Dyad Dashboard</h1>
  </div>
  <button class="new-project-button container" onclick={new_project}
    ><h1>＋ Add Project</h1></button
  >
</div>

<div class="container project-container">
  {#if linked}
    {#each Array.from({ length: 9 }) as _}
      <ProjectEntry project_name="Project 1" friend_name="John" />
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
    margin-top: 5vh;
    color: var(--text-primary);
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
  .project-container {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    align-content: flex-start;
    gap: 16px;
    width: 100%;
    min-height: 60vh;
    margin-top: 5vh;
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
