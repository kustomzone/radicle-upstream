import * as api from "./api";
import * as event from "./event";
import * as identity from "./identity";
import * as remote from "./remote";

// TYPES
export interface Session {
  identity?: identity.Identity;
}

// STATE
const sessionStore = remote.createStore<Session>();
export const session = sessionStore.readable;

// EVENTS
enum Kind {
  Fetch = "FETCH",
}

interface Fetch extends event.Event<Kind> {
  kind: Kind.Fetch;
}

type Msg = Fetch;

const update = (msg: Msg): void => {
  switch (msg.kind) {
    case Kind.Fetch:
      sessionStore.loading();
      api.get<Session>(`session`)
        .then(sessionStore.success)
        .catch(sessionStore.error)

      break;
  }
}

export const fetch = event.create<Kind, Msg>(Kind.Fetch, update);