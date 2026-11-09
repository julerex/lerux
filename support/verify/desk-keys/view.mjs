export const title = "A set-1 letter after the extended prefix";

export const state = (v) => {
  if (v.stroke === 0) return "Ready for a key";
  if (v.stroke === 1) return "Extended prefix";
  if (v.stroke === 2 && v.extended) return "Prefix still held";
  if (v.stroke === 2) return "Prefix consumed";
  if (v.letter) return "Letter a";
  return "The letter was swallowed";
};

export const step = (action) => {
  if (action === "YouPressThePrefix") return "you press the extended prefix";
  if (action === "YouPressTheNextByte") return "you press the next byte";
  if (action === "YouPressA") return "you press A";
  return null;
};

export const group = (v) => {
  if (v.stroke === 0) return "Idle";
  if (v.stroke === 3) return "Typed";
  return "Prefix";
};

export const colors = {
  Idle: "blue",
  Prefix: "orange",
  Typed: "teal",
};
