<script>
  import { Icon } from "../Primitive";
  import Option from "./Dropdown/Option.svelte";

  export let placeholder = null;

  export let dataCy = null;
  export let options = null;
  export let style = null;
  export let optionStyle = null;
  export let valid = true;
  export let validationMessage = null;
  export let validationPending = false;

  let expanded = false;

  // bind to this prop from the outside
  export let value = null;
  export let disabled = false;

  const toggleMenu = () => {
    if (disabled) {
      return;
    }

    expanded = !expanded;
  };
  const hideMenu = () => {
    expanded = false;
  };

  const optionSelectedHandler = event => {
    value = event.detail.value;
    toggleMenu();
  };

  const disabledColor = () => {
    return disabled
      ? "var(--color-foreground-level-4)"
      : "var(--color-foreground-level-6)";
  };

  $: optionByValue = options.find(option => option.value === value);
</script>

<style>
  .dropdown {
    position: relative;
    cursor: pointer;
  }

  .button {
    height: 40px;
    background-color: var(--color-background);
    border: 1px solid var(--color-foreground-level-3);
    border-radius: 4px;
    display: flex;
    align-items: center;
    user-select: none;
    display: flex;
    justify-content: space-between;
    overflow: hidden; /* hack to make inner option corners rounded */
  }

  .button:hover {
    background-color: var(--color-foreground-level-1);
    color: var(--color-foreground);
  }

  .button.disabled {
    cursor: not-allowed;
    box-shadow: 0px 0px 0px 0px;
    background-color: var(--color-background);
  }

  .menu {
    position: absolute;
    top: 0px;
    left: 0px;
    box-shadow: var(--elevation-medium);
    border: 1px solid var(--color-foreground-level-3);
    border-radius: 4px;
    user-select: none;
    background-color: var(--color-background);
    overflow: hidden; /* hack to make inner option corners rounded */
    z-index: 1;
    margin-bottom: 24px;
  }

  .validation-row {
    display: flex;
    align-items: center;
    margin-top: 12px;
    margin-left: 12px;
  }

  .button.invalid {
    box-shadow: 0 0 0 1px var(--color-negative);
    border: 1px solid var(--color-negative);
  }
</style>

<svelte:window on:click={hideMenu} />

<div data-cy={dataCy} class="dropdown" {style}>
  <div
    class="button"
    class:invalid={!valid}
    class:disabled
    on:click|stopPropagation={toggleMenu}>
    {#if value && optionByValue}
      <Option {...optionByValue} {disabled} />
    {:else}
      <p style={`margin: 0 42px 0 12px; color: ${disabledColor()}`}>
        {placeholder}
      </p>
    {/if}
    <Icon.ChevronUpDown
      style={`position: absolute; top: 8px; right: 8px; fill: ${disabledColor()}`} />
  </div>

  <div class="menu" hidden={!expanded}>
    {#each options as option}
      <Option
        style={optionStyle}
        {...option}
        on:selected={optionSelectedHandler}
        selected={value === option.value} />
    {/each}
  </div>

  {#if !validationPending && !valid && validationMessage}
    <div class="validation-row">
      <p style="color: var(--color-negative); text-align: left;">
        {validationMessage}
      </p>
    </div>
  {/if}
</div>
