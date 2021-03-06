<script>
  import { getContext } from "svelte";
  import Router, { push } from "svelte-spa-router";

  import { isDev } from "../../native/ipc.js";
  import { fetch, org as store } from "../src/org.ts";
  import * as path from "../src/path.ts";

  import { Icon } from "../DesignSystem/Primitive";
  import {
    AdditionalActionsDropdown,
    Header,
    HorizontalMenu,
    Remote,
    SidebarLayout,
  } from "../DesignSystem/Component";

  import Fund from "./Org/Fund.svelte";
  import Members from "./Org/Members.svelte";
  import Projects from "./Org/Projects.svelte";

  export let params = null;
  const session = getContext("session");

  const routes = {
    "/orgs/:id": Projects,
    "/orgs/:id/fund": Fund,
    "/orgs/:id/members": Members,
    "/orgs/:id/projects": Projects,
  };

  import ProjectsMenu from "./Org/ProjectsMenu.svelte";
  import MembersMenu from "./Org/MembersMenu.svelte";

  const menuRoutes = {
    "/orgs/:id/": ProjectsMenu,
    "/orgs/:id/projects": ProjectsMenu,
    "/orgs/:id/members": MembersMenu,
  };

  const topbarMenuItems = orgId => {
    const items = [
      {
        icon: Icon.ChevronLeftRight,
        title: "Projects",
        href: path.orgProjects(orgId),
        looseActiveStateMatching: true,
      },
    ];
    isDev() &&
      items.push({
        icon: Icon.Wallet,
        title: "Wallet",
        href: path.orgFund(orgId),
        looseActiveStateMatching: false,
      });
    items.push({
      icon: Icon.User,
      title: "Members",
      href: path.orgMembers(orgId),
      looseActiveStateMatching: false,
    });
    return items;
  };

  let registerProjectMenuItem;
  let sendFundsMenuItem;

  $: dropdownMenuItems = [
    registerProjectMenuItem,
    {
      title: "Add member",
      icon: Icon.Plus,
      event: () => push(path.memberRegistration(params.id)),
    },
    sendFundsMenuItem,
  ];

  if (session.permissions.registerProject) {
    registerProjectMenuItem = {
      dataCy: "add-project",
      title: "Add project",
      icon: Icon.Plus,
      event: () => push(path.registerProject(params.id)),
    };
  } else {
    registerProjectMenuItem = {
      dataCy: "add-project",
      title: "Add project",
      icon: Icon.Plus,
      disabled: true,
      tooltip: "To unlock project registration, create a local project first.",
    };
  }

  if (isDev()) {
    sendFundsMenuItem = {
      title: "Send funds",
      icon: Icon.ArrowUp,
      event: () => console.log("event(send-funds-to-org)"),
    };
  }

  $: fetch({ id: params.id });
</script>

<SidebarLayout dataCy="org-screen" style="margin-top: 0;">
  <Remote {store} let:data={org}>
    <Header.Large variant="org" entity={org}>
      <div slot="left">
        <HorizontalMenu items={topbarMenuItems(params.id)} />
      </div>
      <div slot="right" style="display: flex">
        <Router routes={menuRoutes} />
        <AdditionalActionsDropdown
          dataCy="context-menu"
          style="margin: 0 16px"
          headerTitle={org.shareableEntityIdentifier}
          menuItems={dropdownMenuItems} />
      </div>
    </Header.Large>
    <Router {routes} />
  </Remote>
</SidebarLayout>
