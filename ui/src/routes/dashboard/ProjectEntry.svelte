<script lang="ts">
  import "$lib/palette.css";
  let { project_name, friend_name }: Props = $props();
  interface Props {
    project_name: string;
    friend_name: string;
  }
</script>

<div class="project-entry">
  <p class="project-entry-name">{project_name}</p>
  <p class="project-friend-name">With {friend_name}</p>
  <p>Hackatime Hours Logged:</p>
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
</style>
