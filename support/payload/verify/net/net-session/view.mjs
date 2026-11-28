export const title = "Fetching a page and serving one";

export const state = (v) => {
  if (v.connecting && v.listening) return "Connecting and listening";
  if (v.last === "deconfigured" && v.place !== "bare") return "Address taken from a dropped link";
  if (v.place === "again") return "A second page is served";
  if (v.sent === "again") return "The datagram is sent again";
  if (v.place === "bare" && v.last === "deconfigured") return "Link dropped, still no address";
  if (v.place === "bare") return "Waiting for an address";
  if (v.place === "address") return "Address installed";
  if (v.place === "connecting") return "Connecting";
  if (v.place === "open") return "Connected";
  if (v.place === "fetched") return "The page is fetched";
  if (v.place === "listening") return "Listening";
  if (v.place === "served") return "One page served";
  if (v.place === "held") return "Further requests stop";
  return "In progress";
};

export const step = (action, from) => {
  if (action === "TheOfferArrives") return "an address is offered";
  if (action === "TheLinkDrops" && from.last !== "deconfigured") return "the link drops";
  if (action === "YouConnect") return "you connect";
  if (action === "ThePeerAccepts") return "the peer accepts";
  if (action === "TheBodyArrives") return "the page body arrives";
  if (action === "YouListen") return "you listen";
  if (action === "ThePageIsServed") return "the page is served";
  if (action === "AnotherRequest") return "another request arrives";
  if (action === "YouFinish") return "you close the socket";
  return null;
};

export const group = (v) => {
  if (v.place === "fetched") return "Fetched";
  if (v.place === "listening" || v.place === "served" || v.place === "held" || v.place === "again") {
    return "Serving";
  }
  if (v.place === "connecting" || v.place === "open") return "Fetching";
  return "Address";
};

export const colors = {
  Address: "blue",
  Fetching: "orange",
  Fetched: "teal",
  Serving: "purple",
};
