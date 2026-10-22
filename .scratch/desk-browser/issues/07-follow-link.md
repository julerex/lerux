# Does the spec include following a link?

Type: grilling
Label: wayfinder:grilling
Status: resolved

## Question

[What counts as seeing a page?](03-page-class.md) decided that one navigation to the typed address satisfies the spec. Following a link was left out of that decision. Signing in, granting a permission, and a second surface are out of the spec.

Does this spec include activating a link on that page and showing the public page it names, on the same surface?

## Answer

Activating a link loads the public page it names, on this surface, under the same load rules as the address line. While the page is focused, Enter activates the focused link and the address line shows the `https` address that will be loaded. If no link is focused, Enter does not navigate. Other keys still go to the page.

A link with no host is resolved against the current page's address first. A link that only changes the fragment does not start a fetch. The address line shows the new fragment, and the page scrolls to it. A link that asks for a new window or tab loads on this surface. A `javascript` link runs as script and does not start a load.

The new page replaces the current one. There is no back stack. A page taller than 1024 by 752 scrolls inside that rectangle, including the first page. It is not scaled to fit.

A failed load still follows the address-line failure rules. Submitting a form is outside this decision.
