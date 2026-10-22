# Does the spec include submitting a form?

Type: grilling
Label: wayfinder:grilling
Status: resolved

## Question

[Does the spec include following a link?](07-follow-link.md) decided that Enter on a focused link loads another public page on this surface. Submitting a form was left out of that decision. There is no back stack, no second surface, and no pointer. The Phase 82 keyboard is the only input.

Does this spec include submitting a form on that page and showing the public page the response names, on the same surface?

## Answer

Submitting a form loads the public page the response names, on this surface, under the same load rules as a link. The new page replaces the current one. There is no back stack. A failed load follows the address-line failure rules.

Focus is one control. Tab moves to the next link or control, wraps from the last to the first, and skips a control the stable Firefox and Chrome would skip. A page loads with nothing focused unless the page asks for that focus.

A single-line text field accepts a character when those browsers would, and refuses it when they would. Backspace, Left, and Right edit it. There is no clipboard paste. A long field scrolls horizontally so the cursor stays visible. Enter in a multiline text field inserts a newline. Up and Down change a focused drop-down list. Space toggles a checkbox. Space, Up, or Down selects a radio button and clears the others in its group. Space toggles a focused row of a multi-select list.

Enter submits the focused control's form when those browsers would both submit it. That is a single-line text field whose form has a submit button, or a form with only one single-line text field. The button they would activate is included. Space or Enter on a focused submit button submits that form and includes that button. Space or Enter on a focused reset button restores the form's initial values. Any other focused button does neither. When those browsers would not submit, the page stays as it is.

The form's action is resolved the same way as a link. An empty action is the current page's address. A GET form puts the fields in the query. A POST form sends the body those browsers would send. The address line shows the `https` address that will be loaded, and after the load it shows the final address, including a redirect. A fragment-only action scrolls to that fragment and does not fetch. A `javascript` action runs as script and does not load. A target that asks for a new window loads on this surface. An `http` action, or a host that is not a public name, is rejected the way the address line rejects it. When the page's script cancels the submit, the script runs and the page stays.

A password field is shown and ignores keys. A form that contains one stays as it is. A file field is shown and stays empty, and the submit includes the other fields. When those browsers would refuse the submit because a file is required, the page stays. A disabled control is left out of the submit.
