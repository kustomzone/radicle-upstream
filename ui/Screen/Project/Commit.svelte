<script>
  import { pop } from "svelte-spa-router";
  import { format } from "timeago.js";

  import * as notification from "../../src/notification.ts";
  import { commit as store, fetchCommit } from "../../src/source.ts";
  import * as remote from "../../src/remote.ts";

  import { Flex, Icon } from "../../DesignSystem/Primitive";
  import { Header, Remote } from "../../DesignSystem/Component";

  import FileDiff from "../../DesignSystem/Component/SourceBrowser/FileDiff.svelte";

  export let params = null;
  const projectId = params.id;
  const peerId = params.peerId;
  const commitHash = params.hash;

  $: if ($store.status === remote.Status.Error) {
    console.log($store.error);
    notification.error("Could not fetch commit");
  }

  fetchCommit({ projectId, peerId, sha1: commitHash });
</script>

<style>
  .commit-page {
    max-width: var(--content-max-width);
    margin: 0 auto;
    padding: var(--content-padding);
    min-width: var(--content-min-width);
  }

  .content {
    background: var(--color-foreground-level-1);
    border-radius: 4px;
    padding: 1.5rem;
  }

  .field {
    color: var(--color-foreground-level-6);
    margin-bottom: 0.5rem;
  }

  .field:last-child {
    margin-bottom: 0;
  }

  .branch {
    margin: 0 0.5rem;
    color: var(--color-foreground-level-6);
  }

  .author {
    color: var(--color-foreground);
  }

  .changeset-summary {
    margin-top: 2rem;
    margin-bottom: 1.5rem;
    margin-left: 1.5rem;
  }

  .changeset-summary .additions {
    color: var(--color-positive);
  }

  .changeset-summary .deletions {
    color: var(--color-negative);
  }

  .file-header {
    height: 3rem;
    display: flex;
    align-items: center;
    background: none;
    border-bottom: 1px solid var(--color-foreground-level-3);
    border-radius: 0;
    padding: 0.75rem;
  }

  .file-header:last-child {
    border-bottom: none;
    margin-bottom: 1rem;
  }

  .file-header .diff-type.created {
    margin-left: 1rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    color: var(--color-positive);
    background-color: var(--color-positive-level-1);
  }

  /* TODO(cloudhead): These should be global */
  hr {
    border: 0;
    border-top: 1px solid var(--color-foreground-level-3);
    margin: 1rem 0 1.5rem 0;
  }
</style>

<div class="commit-page" data-cy="commit-page">
  <Remote {store} let:data={commit}>
    <Header.Back style="padding: 1rem;" on:arrowClick={() => pop()}>
      <h3 style="margin-bottom: .75rem">{commit.header.summary}</h3>
      <div class="metadata">
        <span class="field">
          <!-- NOTE(cloudhead): These awful margin hacks are here because
            there is a bug in prettier that breaks our HTML if we try to format
            it differently. -->
          <span>{commit.header.author.name}</span>
          <span>committed</span>
          <span class="typo-mono">{commit.header.sha1.substring(0, 7)}</span>
          <span style="margin-right: -1ch">to</span>
          <span class="branch typo-semi-bold">
            <Icon.Branch
              style="vertical-align: bottom; fill:
              var(--color-foreground-level-6)" />
            <span style="margin-left: -0.5ch">{commit.branch}</span>
          </span>
          <span style="margin-left: -0.5ch">
            {format(commit.header.committerTime * 1000)}
          </span>
        </span>
      </div>
    </Header.Back>
    <div class="content" data-cy="commit-header">
      <pre
        class="typo-mono"
        style="margin-bottom: 1rem">
        {commit.header.summary}
      </pre>
      <pre
        class="description"
        style="margin-bottom: 1rem">
        {commit.header.description}
      </pre>
      <hr />
      <Flex style="align-items: flex-end">
        <div slot="left">
          <p class="field">
            Authored by <span class="author typo-semi-bold"> {commit.header.author.name} </span>
            <span class="typo-mono">&lt;{commit.header.author.email}&gt;</span>
          </p>
          {#if commit.header.committer.email != commit.header.author.email}
            <p class="field">
              Committed by <span class="author typo-semi-bold"> {commit.header.committer.name} </span>
              <span class="typo-mono">
                &lt;{commit.header.committer.email}&gt;
              </span>
            </p>
          {/if}
        </div>
        <div slot="right">
          <!-- TODO(cloudhead): Commit parents when dealing with merge commit -->
          <p class="field">
            Commit <span class="hash">{commit.header.sha1}</span>
          </p>
        </div>
      </Flex>
    </div>

    <main>
      <div class="changeset-summary">
        {#if commit.diff.modified.length > 0}
          <span class="typo-semi-bold">
            {commit.diff.modified.length} file(s) changed
          </span> with <span class="additions typo-semi-bold">
            {commit.stats.additions} additions
          </span> and <span class="deletions typo-semi-bold">
            {commit.stats.deletions} deletions
          </span>
        {/if}
      </div>
      <div>
        {#each commit.diff.created as path}
          <header class="file-header">
            <Icon.File style="margin-right: 8px;" />
            <p class="typo-text-bold">{path}</p>
            <span class="diff-type created">created</span>
          </header>
        {/each}
      </div>
      {#each commit.diff.modified as file}
        <FileDiff {file} />
      {/each}
    </main>
  </Remote>
</div>
