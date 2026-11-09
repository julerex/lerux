export const title = "Menu, maths, browser, and source";

export const state = (v) => {
  if (v.phase === "browser") return "Stuck in the browser";
  if (v.sourcing === "nested") return "A nested source ran";
  if (v.phase === "maths" && v.last === "quit") return "Quit left you in maths";
  if (v.sourcing === "rejected") return "Nested source rejected";
  if (v.sourcing === "open") return "Running a source file";
  if (v.phase === "painting") return "Painting the page";
  if (v.phase === "answered") return "Maths answer";
  if (v.phase === "maths") return "Maths prompt";
  return "Menu";
};

export const step = (action) => {
  if (action === "YouChooseMaths") return "you choose maths";
  if (action === "YouQuitMaths") return "you quit";
  if (action === "YouTypeExpression") return "you type an expression";
  if (action === "YouSeeThePrompt") return "the prompt returns";
  if (action === "YouChooseBrowser") return "you choose browser";
  if (action === "ThePaintReturns") return "the page is painted";
  if (action === "YouSourceAFile") return "you source a file";
  if (action === "YouSourceAgain") return "you source again";
  if (action === "TheNestedLineIsRejected") return "the nested source is rejected";
  if (action === "TheSourceFinishes") return "the source file finishes";
  return null;
};

export const group = (v) => {
  if (v.sourcing === "open" || v.sourcing === "rejected" || v.sourcing === "nested") return "Source";
  if (v.phase === "maths" || v.phase === "answered") return "Maths";
  if (v.phase === "painting" || v.phase === "browser") return "Browser";
  return "Menu";
};

export const colors = {
  Menu: "blue",
  Maths: "orange",
  Browser: "teal",
  Source: "purple",
};
