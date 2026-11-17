# Standalone ICP subnet map widget

A small browser-only adaptation of the three-state widget. It uses no Event
Horizon application modules, backend clients, configuration, assets, bundler or
package dependency. **Licensing remains pending; see below before reusing it.**

## Integration

Copy `subnet-map-widget.js` and `subnet-map-widget.css` beside your webpage, then
include this markup. The only required configuration value is a subnet principal:

```html
<div data-ic-subnet-map="pzp6e-ekpqk-3c5x7-2h6so-njoeq-mt45d-h3h6c-q3mxf-vpeq5-fk5o7-yae"></div>
<link rel="stylesheet" href="subnet-map-widget.css">
<script src="subnet-map-widget.js" defer></script>
<noscript>
  <a href="https://dashboard.internetcomputer.org/network/subnets"
     target="_blank" rel="noopener noreferrer">View subnet maps on ICP Dashboard</a>
</noscript>
```

Use the canonical lowercase principal from ICP Dashboard. The initializer checks
base32 grouping, length, padding and CRC32, then safely encodes it. This does not
prove that the principal exists or identifies a subnet. The origin is fixed to
`https://dashboard.internetcomputer.org`; arbitrary iframe URLs are not accepted.
The no-JavaScript link opens the subnet directory because JavaScript cannot read
configuration when disabled; you may supply a static direct subnet link instead.

Place the host directly under `body`, outside transformed or clipped containers,
so the fixed tab remains attached to the viewport.

The deferred script initializes automatically. `window.initICSubnetMap()` is the
single entry point if initialization is needed later. It returns an object with
`dispose()`. Repeated initialization of the same host returns the existing object;
one widget per page is supported. Disposal cancels loading timers, disconnects the
observer, removes listeners and releases the dialog/frame. Initialization can then
be repeated. It is unnecessary to call the initializer during mode changes.

## Interaction and resources

Fresh pages start **Minimised**: a 46px left-edge tab, vertical title, Restore
(stacked squares) and Maximise (single square). No Dashboard URL is assigned and
no loading timer starts until the first Restore or Maximise.

**Thumbnail** has Minimise (dash) and Maximise controls. Its genuine 600×520px
iframe is scaled uniformly to 25%, giving a 150×130px preview. The map area also
activates the same Maximise button. The preview is inert and excluded from
keyboard/accessibility navigation.

**Expanded** is a native modal with Minimise and Restore controls. Escape received
by the parent returns to the previous non-modal state. Mode changes retain the
same iframe and do not reassign its URL. There is no dismiss action or persistent
preference. A fresh page load starts Minimised again.

Desktop modal width is `min(66.667vw, 1200px)`; below 900px it uses the available
viewport minus safe-area margins. The toolbar stays visible while the modal
scrolls. Widths above 760px uniformly scale a 760×520px layout and reserve its
complete height; smaller widths use the Dashboard's responsive native layout.
Safe-area padding protects left-edge controls without a visible viewport gutter.
Exceptionally short or zoomed viewports allow the non-modal panel to scroll
without shrinking its button targets. No presentation animation is required,
including with reduced motion enabled.

The initial empty document is ignored. The loading layer remains dark, settles
500ms after the external document's load event, and provides a slow-loading
explanation after 12 seconds. A late load recovers. Document load is not proof
that Dashboard data is ready; the external link remains available in the modal.

After activation, the full independently maintained Dashboard application runs:
scaling does not reduce its JavaScript, data-fetching or network cost. Minimising
hides rendering but does **not** guarantee suspension of scripts, timers or
network requests. There is no attempt to control its internal application.

## Security and accessibility boundary

An existing host CSP must add the approved origin to its frame policy, preserving
all other restrictions. For example:

```text
frame-src https://dashboard.internetcomputer.org
```

Keep any other existing allowed origins and directives that your site requires.
The demo has its own restrictive self-hosted script/style policy. Do not replace
a production CSP wholesale with this example. Serve the two files from your own
origin; no external script or proxy is required.

The iframe retains `allow-scripts allow-same-origin allow-popups
allow-popups-to-escape-sandbox` and `referrerpolicy="no-referrer"`. The same-origin
permission refers to Dashboard's origin, not access to your page. Popups may
escape the sandbox and the host's frame-origin restriction. The embedded
Dashboard must continue to use explicit `noopener noreferrer`; this permission
set cannot enforce opener isolation for arbitrary future Dashboard code. The
parent fallback independently supplies those link attributes.

Native dialog semantics isolate the modal background. Non-modal mode changes
focus a visible control; startup does not autofocus. Native browsers handle the
focus trap. Keyboard Escape inside a cross-origin frame is not guaranteed to
reach the parent, so both toolbar controls remain available. Dashboard controls
are hidden from keyboard and assistive navigation until Expanded. Third-party
content, its data, accessibility and future behavior remain outside the host's
certification/control. Wide proportional scaling can offset browser magnification;
the external link permits independent Dashboard zoom.

## Run and validate independently

From this directory, run `python3 -m http.server 8080` and open
`http://localhost:8080/`. Only this directory's HTML, JavaScript and CSS are
needed. No Event Horizon application server or canister is involved.

Check that there are zero Dashboard requests before activation, one embed
document request on the first Restore or Maximise, and none added by later mode
changes. Try all six transitions, Escape from both entry states, keyboard focus,
mobile width, enlarged text and the no-JavaScript fallback. In Expanded, exercise
country/data-centre controls and the full-Dashboard link; confirm its new tab has
no `window.opener`. The repository's development-only
`node tools/scripts/validate-subnet-example.mjs --live` runs a small independent
empty-page fixture and genuine Dashboard smoke check using the already established
Playwright tooling; it is not needed by copied files.

## Licensing decision pending

The repository has no recognised top-level open-source licence. Public visibility
does not grant unrestricted reuse rights. This example proposes **MIT or 0BSD** as
a suitable future arrangement, subject to explicit repository-owner approval.
**No new licence is granted or claimed here, and no licence file has been added.**
Obtain the owner's permission before redistribution or reuse; the integration
instructions describe the technical process, not a grant of rights. This pending
distribution decision does not affect the production frontend's functionality.
