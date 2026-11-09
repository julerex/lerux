const stopped = {
  scan: "Stopped while scanning",
  auth: "Stopped while authenticating",
  assoc: "Stopped during association",
  eapol: "Stopped during the handshake",
  dhcp: "Stopped while asking for an address",
  ping: "Stopped while pinging",
};

const running = {
  scan: "Scanning",
  auth: "Authenticating",
  assoc: "Associated",
  eapol: "Handshake",
  dhcp: "Asking for an address",
  ping: "Pinging",
};

export const title = "Choosing a network and joining it";

export const state = (v) => {
  if (!v.booted && v.radio !== "online") return "Loading firmware";
  if (v.radio === "online" && v.via !== "ok") return "Online without finishing";
  if (v.radio === "online") return "Ping answered";
  if (v.screen === "radio" && v.pass === "short") return "Radio started by a short passphrase";
  if (v.via === "fail") return stopped[v.radio] ?? "Stopped";
  if (v.screen === "ssid") return "Which network";
  if (v.screen === "psk") return "Passphrase";
  if (v.screen === "radio") return running[v.radio] ?? "On the radio";
  return "Menu";
};

export const step = (action, from) => {
  if (action === "TheFirmwareFinishes") return "the firmware finishes";
  if (action === "YouChooseWifi") return "you choose wifi";
  if (action === "YouQuit") return "you quit";
  if (action === "YouNameTheNetwork") return "you name the network";
  if (action === "YouTypeAShortPassphrase") return "you type a short passphrase";
  if (action === "YouTypeAPassphrase") return "you type the passphrase";
  if (action === "Advance" && from.radio === "scan") return "the access point answers";
  if (action === "Advance" && from.radio === "auth") return "the access point associates";
  if (action === "Advance" && from.radio === "assoc") return "the handshake starts";
  if (action === "Advance" && from.radio === "eapol") return "the handshake finishes";
  if (action === "Advance" && from.radio === "dhcp") return "an address arrives";
  if (action === "ThePingAnswers") return "the ping is answered";
  if (action === "TheRadioStops") return "that step fails";
  return null;
};

export const group = (v) => {
  if (v.radio === "online" && v.via === "ok") return "Online";
  if (v.via === "fail" || (v.radio === "online" && v.via !== "ok")) return "Stopped";
  if (v.screen === "radio" || !v.booted) return "Joining";
  return "Choosing";
};

export const colors = {
  Choosing: "blue",
  Joining: "orange",
  Online: "teal",
  Stopped: "tan",
};
