<script>
  import { Avatar, Icon } from "../../Primitive";
  import Urn from "../Urn.svelte";

  export let style = null;
  export let entity = null;
  export let variant = null; // profile | org

  let name;
  if (variant === "profile") {
    if (entity.registered) {
      name = entity.registered;
    } else {
      name = entity.metadata.handle;
    }
  } else if (variant === "org") {
    name = entity.id;
  }
</script>

<style>
  .header {
    display: flex;
    flex-direction: column;
    width: calc(100vw - var(--sidebar-width));
    height: var(--bigheader-height);
    left: var(--sidebar-width);
    z-index: 2;
  }
  .banner {
    height: 200px;
    background-color: var(--color-foreground-level-1);
  }

  .banner-content {
    display: flex;
    justify-content: space-between;
    max-width: var(--content-max-width);
    width: 100%;
    margin: 0 auto;
    padding: 40px var(--content-padding);
  }

  .left {
    display: flex;
  }

  .metadata {
    display: flex;
    flex-direction: column;
    align-self: center;
  }

  .user {
    display: flex;
    align-items: center;
  }

  .shareable-entity-identifier {
    display: flex;
    align-items: center;
    padding-top: 4px;
  }

  .action-bar {
    display: flex;
    justify-content: space-between;
    height: var(--topbar-height);
    width: 100%;
    max-width: var(--content-max-width);
    padding: 0 var(--content-padding);
    margin: 0 auto;
    align-items: center;
    background-color: var(--color-background);
  }

  .banner-action {
    display: flex;
    align-items: center;
  }
</style>

<div data-cy="header" class="header" {style}>
  <div class="banner">
    <div class="banner-content">
      <div class="left">
        <Avatar
          style="margin-right: 32px"
          size="huge"
          variant={variant === 'profile' ? 'circle' : 'square'}
          avatarFallback={entity.avatarFallback} />

        <div class="metadata">
          <div class="user">
            <h1
              data-cy="entity-name"
              style="display: flex; align-items: center;">
              {name}
            </h1>
            {#if variant === 'org' || entity.registered}
              <Icon.Registered
                dataCy="verified-badge"
                style="fill: var(--color-primary); margin-left: 6px;" />
            {/if}
          </div>
          <div class="shareable-entity-identifier">
            <Urn
              urn={entity.shareableEntityIdentifier}
              showCopyOnlyOnHover
              notificationText={`Radicle ID for ${name} copied to your clipboard.`} />
          </div>
        </div>
      </div>
      <div class="banner-action">
        <slot name="top" />
      </div>
    </div>
  </div>
  <div class="action-bar">
    <slot name="left" />
    <slot name="right" />
  </div>
</div>
