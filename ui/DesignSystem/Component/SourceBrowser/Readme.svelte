<script>
  import { Icon, Markdown } from "../../Primitive";
  import { isMarkdown } from "../../../src/source.ts";

  export let content = null;
  export let path = null;
</script>

<style>
  .file-source {
    border: 1px solid var(--color-foreground-level-3);
    border-radius: 3px;
    min-width: var(--content-min-width);
  }

  header .file-header {
    display: flex;
    height: 3rem;
    align-items: center;
    padding-left: 13px;
    color: var(--color-foreground);
    border-bottom: 1px solid var(--color-foreground-level-3);
  }

  header .file-name {
    margin-left: 0.5rem;
  }

  .code {
    padding: 1.5rem;
    overflow-x: auto;
  }

  .container {
    padding: 1rem 2rem;
  }
</style>

<div class="file-source" data-cy="file-source">
  <header>
    <div class="file-header typo-text-bold">
      <Icon.File />
      <span class="file-name">{path}</span>
    </div>
  </header>
  <div class="container">
    {#if isMarkdown(path)}
      <Markdown {content} />
    {:else}
      <pre class="code typo-text-mono">{content}</pre>
    {/if}
  </div>
</div>
